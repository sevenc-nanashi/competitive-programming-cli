# Installation

cpg supports Linux and requires Rust 1.91 or newer when building from source.

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

Tagged releases provide `cpg-x86_64-unknown-linux-gnu.tar.gz` and a SHA-256
checksum on the [releases page](https://github.com/sevenc-nanashi/competitive-programming-cli/releases).
You can install those binaries manually or using package managers like `mise`.

```bash
# Using mise
mise use -g github:sevenc-nanashi/competitive-programming-cli
```

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
