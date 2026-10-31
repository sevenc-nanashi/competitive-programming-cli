# Installation

You can install cpg using cargo:

```bash
# NOTE: Only prereleases are available at this time.

# Build from source
cargo install "competitive-programming-cli@>0.1.0-preview.0"

# Or download the pre-built binaries using cargo-binstall
cargo binstall "competitive-programming-cli@>0.1.0-preview.0"
```

The release workflow builds the following archives, each with a SHA-256 checksum,
for the [releases page](https://github.com/sevenc-nanashi/competitive-programming-cli/releases).

You can install those binaries manually or use [mise](https://mise.jdx.dev) or [packslip](https://packslip.jdx.dev) to install them:

```bash
# Using mise
mise use -g --tool-option prerelease=true "packslip:github.com/sevenc-nanashi/competitive-programming-cli" 

# Using packslip
packslip install github.com/sevenc-nanashi/competitive-programming-cli --pin ps1_iprtl2s5dcfnxce6p5udl6lnhi --version ">0.1.0-preview.0"
```

## Experimental oj backend

For judges without a native backend, install `uv` and make it available on `PATH`.
On first use, cpg runs `uv venv` and installs its [pinned fork of online-judge-api-client](https://github.com/sevenc-nanashi/online-judge-tools-api-client).

See [experimental oj support](./configuration.md#experimental-oj-support) for
usage, virtual environment paths, and limitations.

## Shell completion

`cpg completion <shell>` prints a completion script generated from cpg's command
definitions. It completes commands, aliases, flags, value choices, and file or
directory paths.

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
