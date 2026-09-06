# Installation

cpg supports Linux and requires Rust 1.91 or newer when building from source.

To install the current checkout:

```bash
cargo install --path . --locked
```

After a crates.io release is published, you can install cpg using cargo:

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

Next, [configure your workspace and log in](./configuration.md).
