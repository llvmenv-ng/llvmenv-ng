use llvmenv_ng::{build, config, entry, error::CommandExt, error::Result};

use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use simplelog::{ColorChoice, ConfigBuilder, LevelFilter, SimpleLogger, TermLogger, TerminalMode};
use std::{
    env,
    path::PathBuf,
    process::{exit, Command},
};

#[derive(Debug, Parser)]
#[command(
    name = crate_name!(),
    version = crate_version!(),
    author = crate_authors!(),
    about = crate_description!(),
    long_about = None,
    propagate_version = true,
    arg_required_else_help = true,
)]
enum LLVMEnv {
    #[clap(name = "init", about = "Initialize llvmenv-ng")]
    Init {},

    #[clap(name = "builds", about = "List usable builds")]
    Builds {},

    #[clap(name = "entries", about = "List entries to be built")]
    Entries {},

    #[clap(name = "build-entry", about = "Build LLVM/Clang")]
    BuildEntry {
        name: String,
        #[clap(short, long)]
        update: bool,
        #[clap(short, long, help = "clean build directory")]
        clean: bool,
        #[clap(
            short = 'G',
            long = "builder",
            help = "Overwrite cmake generator setting"
        )]
        builder: Option<String>,
        #[clap(
            short = 'd',
            long = "discard",
            help = "discard source directory for remote resources"
        )]
        discard: bool,
        #[clap(short = 'j', long = "nproc")]
        nproc: Option<usize>,
        #[clap(
            short = 't',
            long = "build-type",
            help = "Overwrite cmake build type (Debug, Release, RelWithDebInfo, or MinSizeRel)"
        )]
        build_type: Option<entry::BuildType>,
    },

    #[clap(name = "current", about = "Show the name of current build")]
    Current {
        #[clap(short, long)]
        verbose: bool,
    },

    #[clap(name = "prefix", about = "Show the prefix of the current build")]
    Prefix {
        #[clap(short, long)]
        verbose: bool,
    },

    #[clap(name = "version", about = "Show the base version of the current build")]
    Version {
        #[clap(short = 'n', long = "name")]
        name: Option<String>,
        #[clap(long = "major")]
        major: bool,
        #[clap(long = "minor")]
        minor: bool,
        #[clap(long = "patch")]
        patch: bool,
    },

    #[clap(name = "global", about = "Set the build to use (global)")]
    Global { name: String },

    #[clap(name = "local", about = "Set the build to use (local)")]
    Local {
        name: String,
        #[clap(short = 'p', long = "path")]
        path: Option<PathBuf>,
    },

    #[clap(name = "archive", about = "archive build into *.tar.xz (require pixz)")]
    Archive {
        name: String,
        #[clap(short, long)]
        verbose: bool,
    },

    #[clap(name = "expand", about = "expand archive")]
    Expand {
        path: PathBuf,
        #[clap(short, long)]
        verbose: bool,
    },

    #[clap(name = "edit", about = "Edit llvmenv-ng configure in your editor")]
    Edit {},

    #[clap(name = "zsh", about = "Setup Zsh integration")]
    Zsh {},
}

fn main() -> Result<()> {
    TermLogger::init(
        LevelFilter::Info,
        ConfigBuilder::new()
            .set_time_offset_to_local()
            .unwrap()
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .or_else(|_| {
        SimpleLogger::init(
            LevelFilter::Info,
            ConfigBuilder::new()
                .set_time_offset_to_local()
                .unwrap()
                .build(),
        )
    })
    .unwrap();

    let opt = LLVMEnv::parse();
    match opt {
        LLVMEnv::Init {} => config::init_config()?,

        LLVMEnv::Builds {} => {
            let builds = build::builds()?;
            let max = builds.iter().map(|b| b.name().len()).max().unwrap();
            for b in &builds {
                println!(
                    "{name:<width$}: {prefix}",
                    name = b.name(),
                    prefix = b.prefix().display(),
                    width = max
                );
            }
        }

        LLVMEnv::Entries {} => match entry::load_entries() {
            Ok(entries) => {
                for entry in &entries {
                    println!("{}", entry.name());
                }
            }
            Err(e) => {
                panic!("{}", e);
            }
        },

        LLVMEnv::BuildEntry {
            name,
            update,
            clean,
            discard,
            builder,
            nproc,
            build_type,
        } => {
            let mut entry = entry::load_entry(&name)?;
            let nproc = nproc.unwrap_or_else(num_cpus::get);

            if let Some(builder) = builder {
                entry.set_builder(&builder)?;
            }

            if let Some(build_type) = build_type {
                entry.set_build_type(build_type)?;
            }

            if discard {
                if let Err(e) = entry.clean_cache_dir() {
                    println!("{}", e);
                }
            }

            if let Err(e) = entry.checkout() {
                println!("{}", e);
            };

            if update {
                if let Err(e) = entry.update() {
                    println!("{}", e);
                };
            }

            if clean {
                if let Err(e) = entry.clean_build_dir() {
                    println!("{}", e);
                };
            }

            if let Err(e) = entry.build(nproc) {
                println!("{}", e);
            };
        }

        LLVMEnv::Current { verbose } => {
            let build = build::seek_build()?;
            println!("{}", build.name());
            if verbose {
                if let Some(env) = build.env_path() {
                    eprintln!("set by {}", env.display());
                }
            }
        }

        LLVMEnv::Prefix { verbose } => {
            let build = build::seek_build()?;
            println!("{}", build.prefix().display());
            if verbose {
                if let Some(env) = build.env_path() {
                    eprintln!("set by {}", env.display());
                }
            }
        }

        LLVMEnv::Version {
            name,
            major,
            minor,
            patch,
        } => {
            let build = if let Some(name) = name {
                get_existing_build(&name)
            } else {
                build::seek_build()?
            };
            let version = build.version()?;
            if !(major || minor || patch) {
                println!("{}.{}.{}", version.major, version.minor, version.patch);
            } else {
                if major {
                    print!("{}", version.major);
                }
                if minor {
                    print!("{}", version.minor);
                }
                if patch {
                    print!("{}", version.patch);
                }
                println!();
            }
        }

        LLVMEnv::Global { name } => {
            let build = get_existing_build(&name);
            build.set_global()?;
        }

        LLVMEnv::Local { name, path } => {
            let build = get_existing_build(&name);
            let path = path.unwrap_or_else(|| env::current_dir().unwrap());
            build.set_local(&path)?;
        }

        LLVMEnv::Archive { name, verbose } => {
            let build = get_existing_build(&name);
            build.archive(verbose)?;
        }

        LLVMEnv::Expand { path, verbose } => {
            build::expand(&path, verbose)?;
        }

        LLVMEnv::Edit {} => {
            let editor = env::var("EDITOR").expect("EDITOR environmental value is not set");
            Command::new(editor)
                .arg(config::config_dir()?.join(config::ENTRY_TOML))
                .check_run()?;
        }

        LLVMEnv::Zsh {} => {
            let src = include_str!("../../llvmenv.zsh");
            println!("{}", src);
        }
    }
    Ok(())
}

fn get_existing_build(name: &str) -> build::Build {
    let build = build::Build::from_name(name).unwrap();
    if build.exists() {
        build
    } else {
        eprintln!("Build '{}' does not exists", name);
        exit(1)
    }
}
