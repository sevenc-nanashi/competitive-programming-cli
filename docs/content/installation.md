# Installation

cpg supports Linux, macOS, and Windows. Building from source requires Rust 1.91 or newer and a native C/C++ toolchain (Xcode Command Line Tools on macOS, Visual Studio Build Tools and NASM on Windows).

To install the current checkout:

```bash
cargo install --path . --locked
```

You can install cpg using cargo:

```bash
# Build from source
cargo install competitive-programming-cli

# Or download the pre-built binaries using cargo-binstall
cargo binstall competitive-programming-cli
```

The release workflow builds the following archives, each with a SHA-256 checksum,
for the [releases page](https://github.com/sevenc-nanashi/competitive-programming-cli/releases).

You can install those binaries manually or using package managers like `mise`.
Each archive includes `command-reference.md` with the generated command reference,
`config.schema.json` with the configuration schema, and `notice.md` with dependency
licenses. All three are also available as separate release downloads.

```bash
# Using mise
mise use -g github:sevenc-nanashi/competitive-programming-cli
```

## Windows

Configuration commands, including setup and language commands, run through
`cmd /C` on Windows. Use CMD syntax, such as `type` instead of `cat` and
`%VARIABLE%` instead of `$VARIABLE`.
Install the compilers and runtimes referenced by your configuration separately.

## Shell completion

`cpg completion <shell>` prints a completion script generated from cpg's command
definitions. It completes commands, aliases, flags, value choices, and file or
directory paths. The script calls cpg itself, so the `usage` executable is not
required.

For Bash, add this to `~/.bashrc`:

```bash
source <(cpg completion bash)
```

For Zsh, add this to `~/.zshrc` after enabling completion:

```zsh
autoload -Uz compinit
compinit
source <(cpg completion zsh)
```

For Fish, save the script in its completion directory:

```fish
mkdir -p ~/.config/fish/completions
cpg completion fish > ~/.config/fish/completions/cpg.fish
```

`elvish`, `nu` (Nushell), and `powershell` are also supported. Save their generated
scripts and load them from your shell configuration. Reopen the shell or source
its configuration to enable completion.

Next, [configure your workspace and log in](./configuration.md).
