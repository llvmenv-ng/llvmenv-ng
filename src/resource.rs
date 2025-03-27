//! Get remote LLVM/Clang source

use futures::{
    executor::{block_on_stream, BlockingStream},
    Stream,
};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info, warn};
use std::{
    fs,
    io::{self, Read},
    path::Path,
    process::Command,
};
use tempfile::TempDir;
use url::Url;

use crate::error::{CommandExt, Error, FileIoConvert, Result};

/// Remote LLVM/Clang resource
#[derive(Debug, PartialEq, Eq)]
pub enum Resource {
    /// Remote Subversion repository
    Svn { url: String },
    /// Remote Git repository
    Git { url: String, branch: Option<String> },
    /// Tar archive
    Tar { url: String },
}

impl Resource {
    /// Detect remote resource from URL
    ///
    /// - Official subversion repository
    ///
    /// ```
    /// # use llvmenv_ng::resource::Resource;
    /// let llvm_official_url = "http://llvm.org/svn/llvm-project/llvm/trunk";
    /// let svn = Resource::from_url(llvm_official_url, &None).unwrap();
    /// assert_eq!(svn, Resource::Svn { url: llvm_official_url.into() });
    /// ```
    ///
    /// - GitHub
    ///
    /// ```
    /// # use llvmenv_ng::resource::Resource;
    /// let github_repo = "https://github.com/llvm/llvm-project";
    /// let git = Resource::from_url(github_repo, &None).unwrap();
    /// assert_eq!(git, Resource::Git { url: github_repo.into(), branch: None });
    /// ```
    ///
    /// - Tar Archive
    ///
    /// ```
    /// # use llvmenv_ng::resource::Resource;
    /// let tar_url = "http://releases.llvm.org/6.0.1/llvm-6.0.1.src.tar.xz";
    /// let tar = Resource::from_url(tar_url, &None).unwrap();
    /// assert_eq!(tar, Resource::Tar { url: tar_url.into() });
    /// ```
    pub fn from_url(url_str: &str, branch: &Option<String>) -> Result<Self> {
        // Check file extension
        if let Ok(filename) = get_filename_from_url(url_str) {
            for extension in &[".tar.gz", ".tar.xz", ".tar.bz2", ".tar.Z", ".tgz", ".taz"] {
                if filename.ends_with(extension) {
                    debug!("Find archive extension '{extension}' at the end of URL");
                    return Ok(Self::Tar {
                        url: url_str.into(),
                    });
                }
            }

            if filename.ends_with("trunk") {
                debug!("Find 'trunk' at the end of URL");
                return Ok(Self::Svn {
                    url: url_str.into(),
                });
            }

            if filename.ends_with(".git") {
                debug!("Find '.git' extension");
                return Ok(Self::Git {
                    url: strip_branch_from_url(url_str)?,
                    branch: branch.as_ref().map_or_else(
                        || get_branch_from_url(url_str).unwrap_or(None),
                        |s| Some(s.clone()),
                    ),
                });
            }
        }

        // Hostname
        let url = Url::parse(url_str).map_err(|_| Error::InvalidUrl {
            url: url_str.into(),
        })?;
        for service in &["github.com", "gitlab.com"] {
            if url.host_str() == Some(service) {
                debug!("URL is a cloud git service: {service}");
                return Ok(Self::Git {
                    url: strip_branch_from_url(url_str)?,
                    branch: get_branch_from_url(url_str)?,
                });
            }
        }

        if url.host_str() == Some("llvm.org") {
            if url.path().starts_with("/svn") {
                debug!("URL is LLVM SVN repository");
                return Ok(Self::Svn {
                    url: url_str.into(),
                });
            }
            if url.path().starts_with("/git") {
                debug!("URL is LLVM Git repository");
                return Ok(Self::Git {
                    url: strip_branch_from_url(url_str)?,
                    branch: branch.as_ref().map_or_else(
                        || get_branch_from_url(url_str).unwrap_or(None),
                        |s| Some(s.clone()),
                    ),
                });
            }
        }

        // Try access with git
        //
        // - SVN repository cannot handle git access
        // - Some Git service (e.g. GitHub) *can* handle svn access
        //
        // ```
        // git init
        // git remote add $url
        // git ls-remote       # This must fail for SVN repo
        // ```
        debug!("Try access with git to {url_str}");
        let tmp_dir = TempDir::new().with("/tmp")?;
        Command::new("git")
            .arg("init")
            .current_dir(tmp_dir.path())
            .silent()
            .check_run()?;
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(url_str)
            .current_dir(tmp_dir.path())
            .silent()
            .check_run()?;
        if matches!(
            Command::new("git")
                .args(["ls-remote"])
                .current_dir(tmp_dir.path())
                .silent()
                .check_run(),
            Ok(())
        ) {
            debug!("Git access succeeds");
            Ok(Self::Git {
                url: strip_branch_from_url(url_str)?,
                branch: branch
                    .clone()
                    .or_else(|| get_branch_from_url(url_str).unwrap().or_else(|| None)),
            })
        } else {
            debug!("Git access failed. Regarded as a SVN repository.");
            Ok(Self::Svn {
                url: url_str.into(),
            })
        }
    }

    pub fn download(&self, dest: &Path) -> Result<()> {
        if !dest.exists() {
            fs::create_dir_all(dest).with(dest)?;
        }
        if !dest.is_dir() {
            return Err(io::Error::new(io::ErrorKind::Other, "Not a directory")).with(dest);
        }

        match self {
            Self::Svn { url, .. } => Command::new("svn")
                .args(["co", url.as_str(), "-r", "HEAD"])
                .arg(dest)
                .check_run()?,
            Self::Git { url, branch } => {
                info!("Git clone {url}");
                let mut git = Command::new("git");
                git.args(["clone", url.as_str(), "-q", "--depth", "1"])
                    .arg(dest);
                if let Some(branch) = branch {
                    git.args(["-b", branch]);
                }
                git.check_run()?;
            }
            Self::Tar { url } => {
                info!("Download Tar file: {url}");
                // This will be large, but at most ~100MB
                let rt = tokio::runtime::Runtime::new()?;
                let bytes = rt.block_on(download(url))?;
                let gz_decoder = flate2::bufread::GzDecoder::new(bytes);
                let mut tar_buf = tar::Archive::new(gz_decoder);
                let entries = tar_buf.entries()?;

                // Iterate through archive contents in order to omit base path component when extracting
                for entry in entries {
                    let mut entry = entry?;
                    let path = entry.path()?;
                    let mut target = dest.to_owned();
                    for comp in path.components().skip(1) {
                        target = target.join(comp);
                    }
                    if let Err(e) = entry.unpack(target) {
                        #[expect(clippy::wildcard_enum_match_arm)]
                        match e.kind() {
                            io::ErrorKind::AlreadyExists => debug!("{e:?}"),
                            _ => warn!("{e:?}"),
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn update(&self, dest: &Path) -> Result<()> {
        match self {
            Self::Svn { .. } => Command::new("svn")
                .arg("update")
                .current_dir(dest)
                .check_run()?,
            Self::Git { .. } => Command::new("git")
                .arg("pull")
                .current_dir(dest)
                .check_run()?,
            Self::Tar { .. } => {}
        }
        Ok(())
    }
}

struct Download<T> {
    stream: T,
    bytes: Option<bytes::Bytes>,
    bar: ProgressBar,
}

impl<T> Drop for Download<T> {
    fn drop(&mut self) {
        self.bar.finish();
    }
}

async fn download(
    url: &str,
) -> Result<Download<BlockingStream<impl Stream<Item = reqwest::Result<bytes::Bytes>>>>> {
    let req = reqwest::get(url).await?;
    let status = req.status();
    if !status.is_success() {
        return Err(Error::HttpError {
            url: url.into(),
            status,
        });
    }

    let bar = if let Some(Ok(Ok(content_length))) = req.headers().get(reqwest::header::CONTENT_LENGTH)
    .map(|h| h.to_str().map(str::parse))
        {
            ProgressBar::new(content_length)
        }
        else {
            ProgressBar::no_length()
        }
        .with_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:38.cyan/blue}] {bytes}/{total_bytes} ({eta}) [{bytes_per_sec}]")
            ?
            .progress_chars("#>-"));

    Ok(Download {
        stream: block_on_stream(req.bytes_stream()),
        bytes: None,
        bar,
    })
}

impl<T> io::Read for Download<T>
where
    T: Iterator<Item = reqwest::Result<bytes::Bytes>>,
{
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let mut bytes = if let Some(bytes) = self.bytes.take() {
            bytes
        } else {
            match self.stream.next() {
                Some(Ok(bytes)) => bytes,
                Some(Err(err)) => return Err(io::Error::new(io::ErrorKind::Other, err)),
                None => return Ok(0),
            }
        };
        if bytes.len() > buf.len() {
            self.bytes = Some(bytes.split_off(buf.len()));
        } else {
            buf = &mut buf[..bytes.len()];
        }
        buf.copy_from_slice(&bytes);
        self.bar.inc(bytes.len() as u64);
        Ok(bytes.len())
    }
}

impl<T> io::BufRead for Download<T>
where
    T: Iterator<Item = reqwest::Result<bytes::Bytes>>,
{
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.bytes.is_none() {
            self.read_exact(&mut [])?;
        }

        Ok(self.bytes.as_ref().map_or(&[], |b| &b[..]))
    }

    fn consume(&mut self, amt: usize) {
        if let Some(mut bytes) = self.bytes.take() {
            if bytes.len() > amt {
                self.bytes = Some(bytes.split_off(amt));
            }
        }
    }
}

fn get_filename_from_url(url_str: &str) -> Result<String> {
    let url = ::url::Url::parse(url_str).map_err(|_| Error::InvalidUrl {
        url: url_str.into(),
    })?;
    let seg = url.path_segments().ok_or(Error::InvalidUrl {
        url: url_str.into(),
    })?;
    let filename = seg.last().ok_or(Error::InvalidUrl {
        url: url_str.into(),
    })?;
    Ok(filename.to_string())
}

fn get_branch_from_url(url_str: &str) -> Result<Option<String>> {
    let url = ::url::Url::parse(url_str).map_err(|_| Error::InvalidUrl {
        url: url_str.into(),
    })?;
    Ok(url.fragment().map(ToOwned::to_owned))
}

fn strip_branch_from_url(url_str: &str) -> Result<String> {
    let mut url = ::url::Url::parse(url_str).map_err(|_| Error::InvalidUrl {
        url: url_str.into(),
    })?;
    url.set_fragment(None);
    Ok(url.into())
}

#[cfg(test)]
#[expect(clippy::panic_in_result_fn)]
mod tests {
    use super::*;

    // Test downloading this repo
    #[test]
    fn test_git_download() -> Result<()> {
        let git = Resource::Git {
            url: "http://github.com/llvmenv-ng/llvmenv-ng".into(),
            branch: None,
        };
        let tmp_dir = TempDir::new().with("/tmp")?;
        git.download(tmp_dir.path())?;
        let cargo_toml = tmp_dir.path().join("Cargo.toml");
        assert!(cargo_toml.exists());
        Ok(())
    }

    #[test]
    fn test_get_filename_from_url() {
        let url = "http://releases.llvm.org/6.0.1/llvm-6.0.1.src.tar.xz";
        assert_eq!(get_filename_from_url(url).unwrap(), "llvm-6.0.1.src.tar.xz");
    }

    #[test]
    fn test_with_git_branches() {
        let github_mirror = "https://github.com/llvm-mirror/llvm";
        let git = Resource::from_url(github_mirror, &None).unwrap();
        assert_eq!(
            git,
            Resource::Git {
                url: github_mirror.into(),
                branch: None
            }
        );
        assert_eq!(
            Resource::from_url("https://github.com/llvm-mirror/llvm#release_80", &None).unwrap(),
            Resource::Git {
                url: "https://github.com/llvm-mirror/llvm".into(),
                branch: Some("release_80".into())
            }
        );
    }
}
