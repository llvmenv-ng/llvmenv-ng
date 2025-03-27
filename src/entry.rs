//! Describes how to compile LLVM/Clang
//!
//! entry.toml
//! -----------
//! **entry** in llvmenv-ng describes how to compile LLVM/Clang, and set by `$XDG_CONFIG_HOME/llvmenv-ng/entry.toml`.
//! `llvmenv-ng init` generates default setting:
//!
//! ```toml
//! [llvm-project]
//! url    = "https://github.com/llvm/llvm-project"
//! target = ["X86"]
//! tools = ["clang", "clang-tools-extra"]
//! ```
//!
//! (TOML format has been changed largely at version 0.2.0)
//!
//! **tools** property means LLVM tools, e.g. clang, compiler-rt, lld, and so on.
//! This toml will be decoded into [`EntrySetting`][EntrySetting] and normalized into [Entry][Entry].
//!
//! [Entry]: ./enum.Entry.html
//! [EntrySetting]: ./struct.EntrySetting.html
//!
//! Local entries (since v0.2.0)
//! -------------
//! Different from above *remote* entries, you can build locally cloned LLVM source with *local* entry.
//!
//! ```toml
//! [my-local-llvm]
//! path = "/path/to/your/src"
//! target = ["X86"]
//! ```
//!
//! Entry is regarded as *local* if there is `path` property, and *remote* if there is `url` property.
//! Other options are common to *remote* entries.
//!
//! Pre-defined entries
//! ------------------
//!
//! There is also pre-defined entries corresponding to the LLVM/Clang releases:
//!
//! ```shell
//! $ llvmenv-ng entries
//! llvm-mirror
//! 20.1.2
//! 20.1.1
//! 20.1.0
//! 19.1.7
//! 19.1.6
//! 19.1.5
//! 19.1.4
//! 19.1.3
//! 19.1.2
//! 19.1.1
//! 19.1.0
//! 18.1.8
//! 18.1.7
//! 18.1.6
//! 18.1.5
//! 18.1.4
//! 18.1.3
//! 18.1.2
//! 18.1.1
//! 18.1.0
//! 17.0.6
//! 17.0.5
//! 17.0.4
//! 17.0.3
//! 17.0.2
//! 17.0.1
//! 17.0.0
//! 16.0.6
//! 16.0.5
//! 16.0.4
//! 16.0.3
//! 16.0.2
//! 16.0.1
//! 16.0.0
//! 15.0.7
//! 15.0.6
//! 15.0.5
//! 15.0.4
//! 15.0.3
//! 15.0.2
//! 15.0.1
//! 15.0.0
//! 14.0.6
//! 14.0.5
//! 14.0.4
//! 14.0.3
//! 14.0.2
//! 14.0.1
//! 14.0.0
//! 13.0.1
//! 13.0.0
//! 12.0.1
//! 12.0.0
//! 11.1.0
//! 11.0.1
//! 11.0.0
//! 10.0.1
//! 10.0.0
//! 9.0.1
//! 9.0.0
//! 8.0.1
//! 8.0.0
//! 7.1.0
//! 7.0.1
//! 7.0.0
//! 6.0.1
//! 6.0.0
//! 5.0.2
//! 5.0.1
//! 5.0.0
//! 4.0.1
//! 4.0.0
//! 3.9.1
//! 3.9.0
//! 3.8.1
//! 3.8.0
//! 3.7.1
//! 3.7.0
//! 3.6.2
//! 3.6.1
//! 3.6.0
//! 3.5.2
//! 3.5.1
//! 3.5.0
//! 3.4.2
//! 3.4.1
//! 3.4.0
//! 3.3.0
//! 3.2.0
//! 3.1.0
//! 3.0.0
//! 2.9.0
//! 2.8.0
//! 2.7.0
//! 2.6.0
//! 2.5.0
//! 2.4.0
//! 2.3.0
//! 2.2.0
//! 2.1.0
//! 2.0.0
//! 1.9.0
//! 1.6.0
//! 1.5.0
//! 1.4.0
//! 1.3.0
//! 1.2.0
//! 1.1.0
//! 1.0.0
//! ```
//!
//! These are compiled with the default setting as shown above. You have to create entry manually
//! if you want to use custom settings.

use itertools::Itertools;
use log::{info, warn};
use semver::{Version, VersionReq};
use serde_derive::Deserialize;
use std::{collections::HashMap, convert::Into, fmt, fs, path::PathBuf, process, str::FromStr};

use crate::{
    config::{cache_dir, config_dir, data_dir, ENTRY_TOML},
    error::{CommandExt, Error, FileIoConvert, Result},
    resource::Resource,
};

/// Option for `CMake` Generators
///
/// - Official document: [CMake Generators](https://cmake.org/cmake/help/latest/manual/cmake-generators.7.html)
///
/// ```
/// # use llvmenv_ng::entry::CMakeGenerator;
/// # use std::str::FromStr;
/// assert_eq!(CMakeGenerator::from_str("Makefile").unwrap(), CMakeGenerator::Makefile);
/// assert_eq!(CMakeGenerator::from_str("Ninja").unwrap(), CMakeGenerator::Ninja);
/// assert_eq!(CMakeGenerator::from_str("vs").unwrap(), CMakeGenerator::VisualStudio);
/// assert_eq!(CMakeGenerator::from_str("VisualStudio").unwrap(), CMakeGenerator::VisualStudio);
/// assert!(CMakeGenerator::from_str("MySuperBuilder").is_err());
/// ```
#[derive(Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub enum CMakeGenerator {
    /// Use platform default generator (without -G option)
    #[default]
    Platform,
    /// Unix Makefile
    Makefile,
    /// Ninja generator
    Ninja,
    /// Visual Studio 15 2017
    VisualStudio,
    /// Visual Studio 15 2017 Win64
    VisualStudioWin64,
}

impl FromStr for CMakeGenerator {
    type Err = Error;
    fn from_str(generator: &str) -> Result<Self> {
        Ok(match generator.to_ascii_lowercase().as_str() {
            "makefile" => Self::Makefile,
            "ninja" => Self::Ninja,
            "visualstudio" | "vs" => Self::VisualStudio,

            _ => {
                return Err(Error::UnsupportedGenerator {
                    generator: generator.into(),
                });
            }
        })
    }
}

impl CMakeGenerator {
    /// Option for `CMake`
    #[must_use]
    pub fn option(&self) -> Vec<String> {
        match self {
            Self::Platform => Vec::new(),
            Self::Makefile => vec!["-G", "Unix Makefiles"],
            Self::Ninja => vec!["-G", "Ninja"],
            Self::VisualStudio => vec!["-G", "Visual Studio 15 2017"],
            Self::VisualStudioWin64 => {
                vec!["-G", "Visual Studio 15 2017 Win64", "-Thost=x64"]
            }
        }
        .into_iter()
        .map(Into::into)
        .collect()
    }

    /// Option for cmake build mode (`cmake --build` command)
    #[must_use]
    pub fn build_option(&self, nproc: usize, build_type: BuildType) -> Vec<String> {
        match self {
            Self::VisualStudioWin64 | Self::VisualStudio => {
                vec!["--config".into(), format!("{:?}", build_type)]
            }
            Self::Platform => Vec::new(),
            Self::Makefile | Self::Ninja => {
                vec!["--".into(), "-j".into(), format!("{}", nproc)]
            }
        }
    }
}

/// `CMake` build type
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildType {
    Debug,
    #[default]
    Release,
    RelWithDebInfo,
    MinSizeRel,
}

impl FromStr for BuildType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "debug" => Ok(Self::Debug),
            "release" => Ok(Self::Release),
            "relwithdebinfo" => Ok(Self::RelWithDebInfo),
            "minsizerel" => Ok(Self::MinSizeRel),

            _ => Err(Error::UnsupportedBuildType {
                build_type: s.to_string(),
            }),
        }
    }
}

impl fmt::Display for BuildType {
    #[expect(clippy::use_debug)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Setting for both Remote and Local entries. TOML setting file will be decoded into this struct.
///
///
#[derive(Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct EntrySetting {
    /// URL of remote LLVM resource, see also [resource](../resource/index.html) module
    pub url: Option<String>,

    /// The relative path to the LLVM source, used if the URL used points to a root project in which the LLVM source is contained.
    /// Note: This is here for feeble future-proofing. The LLVM project mono-repo uses the 'llvm' subdirectory `CMake` config for building all other projects
    pub relative_path: Option<PathBuf>,

    /// Branch to clone if a git URL is used
    pub branch: Option<String>,

    /// Path of local LLVM source dir
    pub path: Option<String>,

    /// Additional LLVM Tools, e.g. clang, openmp, lld, and so on.
    #[serde(default)]
    pub tools: Vec<String>,

    /// Target to be build, e.g. "X86". Empty means all backend
    #[serde(default)]
    pub target: Vec<String>,

    /// `CMake` Generator option (-G option in cmake)
    #[serde(default)]
    pub generator: CMakeGenerator,

    ///  Option for `CMAKE_BUILD_TYPE`
    #[serde(default)]
    pub build_type: BuildType,

    /// Additional LLVM build options
    #[serde(default)]
    pub option: HashMap<String, String>,
}

/// Describes how to compile LLVM/Clang
///
/// See also [module level document](index.html).
#[derive(Debug, PartialEq, Eq)]
pub enum Entry {
    Remote {
        name: String,
        version: Option<Version>,
        url: String,
        tools: Vec<String>,
        relative_path: Option<PathBuf>,
        branch: Option<String>,
        setting: EntrySetting,
    },
    Local {
        name: String,
        version: Option<Version>,
        path: PathBuf,
        setting: EntrySetting,
    },
}

fn load_entry_toml(toml_str: &str) -> Result<Vec<Entry>> {
    let entries: HashMap<String, EntrySetting> = toml::from_str(toml_str)?;
    entries
        .into_iter()
        .map(|(name, setting)| Entry::parse_setting(&name, Version::parse(&name).ok(), setting))
        .collect()
}

#[expect(clippy::too_many_lines)]
#[must_use]
pub fn official_releases() -> Vec<Entry> {
    vec![
        Entry::official(20, 1, 2),
        Entry::official(20, 1, 1),
        Entry::official(20, 1, 0),
        Entry::official(19, 1, 7),
        Entry::official(19, 1, 6),
        Entry::official(19, 1, 5),
        Entry::official(19, 1, 4),
        Entry::official(19, 1, 3),
        Entry::official(19, 1, 2),
        Entry::official(19, 1, 1),
        Entry::official(19, 1, 0),
        Entry::official(18, 1, 8),
        Entry::official(18, 1, 7),
        Entry::official(18, 1, 6),
        Entry::official(18, 1, 5),
        Entry::official(18, 1, 4),
        Entry::official(18, 1, 3),
        Entry::official(18, 1, 2),
        Entry::official(18, 1, 1),
        Entry::official(18, 1, 0),
        Entry::official(17, 0, 6),
        Entry::official(17, 0, 5),
        Entry::official(17, 0, 4),
        Entry::official(17, 0, 3),
        Entry::official(17, 0, 2),
        Entry::official(17, 0, 1),
        Entry::official(17, 0, 0),
        Entry::official(16, 0, 6),
        Entry::official(16, 0, 5),
        Entry::official(16, 0, 4),
        Entry::official(16, 0, 3),
        Entry::official(16, 0, 2),
        Entry::official(16, 0, 1),
        Entry::official(16, 0, 0),
        Entry::official(15, 0, 7),
        Entry::official(15, 0, 6),
        Entry::official(15, 0, 5),
        Entry::official(15, 0, 4),
        Entry::official(15, 0, 3),
        Entry::official(15, 0, 2),
        Entry::official(15, 0, 1),
        Entry::official(15, 0, 0),
        Entry::official(14, 0, 6),
        Entry::official(14, 0, 5),
        Entry::official(14, 0, 4),
        Entry::official(14, 0, 3),
        Entry::official(14, 0, 2),
        Entry::official(14, 0, 1),
        Entry::official(14, 0, 0),
        Entry::official(13, 0, 1),
        Entry::official(13, 0, 0),
        Entry::official(12, 0, 1),
        Entry::official(12, 0, 0),
        Entry::official(11, 1, 0),
        Entry::official(11, 0, 1),
        Entry::official(11, 0, 0),
        Entry::official(10, 0, 1),
        Entry::official(10, 0, 0),
        Entry::official(9, 0, 1),
        Entry::official(9, 0, 0),
        Entry::official(8, 0, 1),
        Entry::official(8, 0, 0),
        Entry::official(7, 1, 0),
        Entry::official(7, 0, 1),
        Entry::official(7, 0, 0),
        Entry::official(6, 0, 1),
        Entry::official(6, 0, 0),
        Entry::official(5, 0, 2),
        Entry::official(5, 0, 1),
        Entry::official(5, 0, 0),
        Entry::official(4, 0, 1),
        Entry::official(4, 0, 0),
        Entry::official(3, 9, 1),
        Entry::official(3, 9, 0),
        Entry::official(3, 8, 1),
        Entry::official(3, 8, 0),
        Entry::official(3, 7, 1),
        Entry::official(3, 7, 0),
        Entry::official(3, 6, 2),
        Entry::official(3, 6, 1),
        Entry::official(3, 6, 0),
        Entry::official(3, 5, 2),
        Entry::official(3, 5, 1),
        Entry::official(3, 5, 0),
        Entry::official(3, 4, 2),
        Entry::official(3, 4, 1),
        Entry::official(3, 4, 0),
        Entry::official(3, 3, 0),
        Entry::official(3, 2, 0),
        Entry::official(3, 1, 0),
        Entry::official(3, 0, 0),
        Entry::official(2, 9, 0),
        Entry::official(2, 8, 0),
        Entry::official(2, 7, 0),
        Entry::official(2, 6, 0),
        Entry::official(2, 5, 0),
        Entry::official(2, 4, 0),
        Entry::official(2, 3, 0),
        Entry::official(2, 2, 0),
        Entry::official(2, 1, 0),
        Entry::official(2, 0, 0),
        Entry::official(1, 9, 0),
        Entry::official(1, 6, 0),
        Entry::official(1, 5, 0),
        Entry::official(1, 4, 0),
        Entry::official(1, 3, 0),
        Entry::official(1, 2, 0),
        Entry::official(1, 1, 0),
        Entry::official(1, 0, 0),
    ]
}

pub fn load_entries() -> Result<Vec<Entry>> {
    let global_toml = config_dir()?.join(ENTRY_TOML);
    let mut entries = load_entry_toml(&fs::read_to_string(&global_toml).with(&global_toml)?)?;
    let mut official = official_releases();
    entries.append(&mut official);
    Ok(entries)
}

pub fn load_entry(name: &str) -> Result<Entry> {
    let entries = load_entries()?;
    for entry in entries {
        if entry.name() == name {
            return Ok(entry);
        }

        if let Some(version) = entry.version() {
            if let Ok(req) = VersionReq::parse(name) {
                if req.matches(version) {
                    return Ok(entry);
                }
            }
        }
    }
    Err(Error::InvalidEntry {
        message: "Entry not found".into(),
        name: name.into(),
    })
}

lazy_static::lazy_static! {
    static ref LLVM_8_0_1: Version = Version::new(8, 0, 1);
    static ref LLVM_9_0_0: Version = Version::new(9, 0, 0);
}

impl Entry {
    /// Entry for official LLVM release
    #[must_use]
    pub fn official(major: u64, minor: u64, patch: u64) -> Self {
        let version = Version::new(major, minor, patch);

        let setting = EntrySetting {
            url: Some(format!(
                "https://github.com/llvm/llvm-project/archive/llvmorg-{version}.tar.gz",
            )),
            tools: vec![
                "clang".into(),
                "lld".into(),
                "lldb".into(),
                "clang-tools-extra".into(),
                "polly".into(),
                "compiler-rt".into(),
                //"libcxx".into(),
                //"libcxxabi".into(),
                //"libunwind".into(),
                "openmp".into(),
            ],
            ..Default::default()
        };

        let name = version.to_string();

        #[expect(clippy::expect_used)]
        Self::parse_setting(&name, Some(version), setting).expect("Failed to parse entry")
    }

    fn parse_setting(name: &str, version: Option<Version>, setting: EntrySetting) -> Result<Self> {
        if setting.path.is_some() && setting.url.is_some() {
            return Err(Error::InvalidEntry {
                name: name.into(),
                message: "One of Path or URL are allowed".into(),
            });
        }

        if let Some(path) = &setting.path {
            if !setting.tools.is_empty() {
                warn!("'tools' must be used with URL, ignored");
            }
            return Ok(Self::Local {
                name: name.into(),
                version,
                path: PathBuf::from(shellexpand::full(&path)?.to_string()),
                setting,
            });
        }

        if let Some(url) = &setting.url {
            return Ok(Self::Remote {
                name: name.into(),
                version,
                url: url.clone(),
                tools: setting.tools.clone(),
                relative_path: None,
                branch: None,
                setting,
            });
        }
        Err(Error::InvalidEntry {
            name: name.into(),
            message: "Path nor URL are not found".into(),
        })
    }

    const fn setting(&self) -> &EntrySetting {
        match self {
            Self::Local { setting, .. } | Self::Remote { setting, .. } => setting,
        }
    }

    fn setting_mut(&mut self) -> &mut EntrySetting {
        match self {
            Self::Local { setting, .. } | Self::Remote { setting, .. } => setting,
        }
    }

    pub fn set_builder(&mut self, generator: &str) -> Result<()> {
        let generator = CMakeGenerator::from_str(generator)?;
        self.setting_mut().generator = generator;
        Ok(())
    }

    pub fn set_build_type(&mut self, build_type: BuildType) -> Result<()> {
        self.setting_mut().build_type = build_type;
        Ok(())
    }

    pub fn checkout(&self) -> Result<()> {
        match self {
            Self::Remote { url, branch, .. } => {
                let src = Resource::from_url(url, branch)?;
                src.download(&self.src_dir()?)?;
            }

            Self::Local { .. } => {}
        }
        Ok(())
    }

    pub fn clean_cache_dir(&self) -> Result<()> {
        let path = self.src_dir()?;
        info!("Remove cache dir: {}", path.display());
        fs::remove_dir_all(&path).with(&path)?;
        Ok(())
    }

    pub fn update(&self) -> Result<()> {
        match self {
            Self::Remote { url, branch, .. } => {
                let src = Resource::from_url(url, branch)?;
                src.update(&self.src_dir()?)?;
            }
            Self::Local { .. } => {}
        }
        Ok(())
    }

    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Local { name, .. } | Self::Remote { name, .. } => name,
        }
    }

    pub fn root_src_dir(&self) -> Result<PathBuf> {
        Ok(match self {
            Self::Remote { name, .. } => cache_dir()?.join(name),

            Self::Local { path, .. } => path.into(),
        })
    }

    #[must_use]
    pub const fn version(&self) -> Option<&Version> {
        match self {
            Self::Local { version, .. } | Self::Remote { version, .. } => version.as_ref(),
        }
    }

    pub fn src_dir(&self) -> Result<PathBuf> {
        Ok(match self {
            Self::Remote { name, .. } => cache_dir()?.join(name),

            Self::Local { path, .. } => path.into(),
        })
    }

    pub fn build_dir(&self) -> Result<PathBuf> {
        let dir = self.src_dir()?.join("build");
        if !dir.exists() {
            info!("Create build dir: {}", dir.display());
            fs::create_dir_all(&dir).with(&dir)?;
        }
        Ok(dir)
    }

    pub fn clean_build_dir(&self) -> Result<()> {
        let path = self.build_dir()?;
        info!("Remove build dir: {}", path.display());
        fs::remove_dir_all(&path).with(&path)?;
        Ok(())
    }

    pub fn prefix(&self) -> Result<PathBuf> {
        Ok(data_dir()?.join(self.name()))
    }

    pub fn build(&self, nproc: usize) -> Result<()> {
        self.configure()?;
        process::Command::new("cmake")
            .args([
                "--build",
                &format!("{}", self.build_dir()?.display()),
                "--target",
                "install",
            ])
            .args(
                self.setting()
                    .generator
                    .build_option(nproc, self.setting().build_type),
            )
            .check_run()?;
        Ok(())
    }

    fn configure(&self) -> Result<()> {
        let setting = self.setting();
        let mut opts = setting.generator.option();
        if let Some(relative_path) = setting.relative_path.as_ref() {
            opts.push(format!("../{}", relative_path.display()));
        } else {
            opts.push("../llvm".into());
        }

        opts.push(format!(
            "-DCMAKE_INSTALL_PREFIX={}",
            data_dir()?.join(self.prefix()?).display()
        ));
        opts.push(format!("-DCMAKE_BUILD_TYPE={}", setting.build_type));

        // Enable ccache if exists
        if which::which("ccache").is_ok() {
            opts.push("-DLLVM_CCACHE_BUILD=ON".into());
        }

        // Enable lld if exists
        if which::which("lld").is_ok() {
            opts.push("-DLLVM_ENABLE_LLD=ON".into());
        }

        // Target architectures
        if !setting.target.is_empty() {
            opts.push(format!(
                "-DLLVM_TARGETS_TO_BUILD={}",
                setting.target.iter().join(";")
            ));
        }

        // Enable selected tools/projects
        opts.push(format!(
            "-DLLVM_ENABLE_PROJECTS={}",
            setting.tools.join(";")
        ));

        // Disable develop warnings
        opts.push("-Wno-dev".to_string());
        opts.push("-Wno-deprecated".to_string());
        opts.push("--no-warn-unused-cli".to_string());

        // Other options
        for (k, v) in &setting.option {
            opts.push(format!("-D{k}={v}"));
        }

        process::Command::new("cmake")
            .args(&opts)
            .current_dir(self.build_dir()?)
            .check_run()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url() {
        let setting = EntrySetting {
            url: Some("http://llvm.org/svn/llvm-project/llvm/trunk".into()),
            ..Default::default()
        };
        let _entry = Entry::parse_setting("url", None, setting).unwrap();
    }

    #[test]
    fn parse_path() {
        let setting = EntrySetting {
            path: Some("~/.config/llvmenv-ng".into()),
            ..Default::default()
        };
        let _entry = Entry::parse_setting("path", None, setting).unwrap();
    }

    #[should_panic]
    #[test]
    fn parse_no_entry() {
        let setting = EntrySetting::default();
        let _entry = Entry::parse_setting("no_entry", None, setting).unwrap();
    }

    #[should_panic]
    #[test]
    fn parse_duplicated() {
        let setting = EntrySetting {
            url: Some("http://llvm.org/svn/llvm-project/llvm/trunk".into()),
            path: Some("~/.config/llvmenv-ng".into()),
            ..Default::default()
        };
        let _entry = Entry::parse_setting("duplicated", None, setting).unwrap();
    }

    #[test]
    fn parse_with_version() {
        let path = "~/.config/llvmenv-ng";
        let version = Version::new(10, 0, 0);
        let setting = EntrySetting {
            path: Some(path.into()),
            ..Default::default()
        };
        let entry = Entry::parse_setting("path", Some(version.clone()), setting.clone()).unwrap();

        assert_eq!(entry.version(), Some(&version));
        assert_eq!(
            entry,
            Entry::Local {
                name: "path".into(),
                version: Some(version),
                path: PathBuf::from(shellexpand::full(path).unwrap().to_string()),
                setting,
            }
        );
    }
}
