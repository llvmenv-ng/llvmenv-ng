# llvmenv-ng

[![crate](https://img.shields.io/crates/v/llvmenv-ng.svg)](https://crates.io/crates/llvmenv-ng)
[![docs.rs](https://docs.rs/llvmenv-ng/badge.svg)](https://docs.rs/llvmenv-ng)

Manage multiple LLVM/Clang build

## Install

0. Install cmake, builder (make/ninja), and C++ compiler (g++/clang++)
1. Install Rust using [rustup](https://github.com/rust-lang-nursery/rustup.rs) or any other method. The minimum supported Rust version is currently **1.81.0**.
2. `cargo install llvmenv-ng`

### Basic Usage

To install a specific version of LLVM after following the installation steps above, run these shell commands ("20.1.2" can be replaced with any other version found with `llvmenv-ng entries`):

```sh
llvmenv-ng init
llvmenv-ng entries
llvmenv-ng build-entry 20.1.2
```

## zsh integration

You can switch LLVM/Clang builds automatically using zsh precmd-hook. Please add a line into your `.zshrc`:

```sh
source <(llvmenv-ng zsh)
```

If `$LLVMENV_RUST_BINDING` environmental value is non-zero, llvmenv-ng exports `LLVM_SYS_60_PREFIX=$(llvmenv-ng prefix)` in addition to `$PATH`.

```sh
export LLVMENVNG_RUST_BINDING=1
source <(llvmenv-ng zsh)
```

This is useful for [llvm-sys.rs](https://github.com/tari/llvm-sys.rs) users. Be sure that this env value will not be unset by llvmenv-ng, only overwrite.

## Concepts

### entry

- **entry** describes how to compile LLVM/Clang
- Two types of entries
  - *Remote*: Download LLVM from Git/SVN repository or Tar archive, and then build
  - *Local*: Build locally cloned LLVM source
- See [the module document](https://docs.rs/llvmenv-ng/*/llvmenv-ng/entry/index.html) for detail

### build

- **build** is a directory where compiled executables (e.g. clang) and libraries are installed.
- They are compiled by `llvmenv-ng build-entry`, and placed at `$XDG_DATA_HOME/llvmenv-ng` (usually `$HOME/.local/share/llvmenv-ng`).
- There is a special build, "system", which uses system's executables.

### global/local prefix

- `llvmenv-ng prefix` returns the path of the current build (e.g. `$XDG_DATA_HOME/llvmenv-ng/llvm-dev`, or `/usr` for system build).
- `llvmenv-ng global [name]` sets default build, and `llvmenv-ng local [name]` sets directory-local build by creating `.llvmenv` text file.
- You can confirm which `.llvmenv` sets the current prefix by `llvmenv-ng prefix -v`.
