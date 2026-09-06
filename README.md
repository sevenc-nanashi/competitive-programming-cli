# Competitive Programming CLI (cpg)

> [!WARNING]
> This project is in early development.
>
> - Configuration and metadata formats may change.
> - Only Linux is supported for now.

cpg is a command-line interface tool for competitive programming.

This tool can:

- Download a problem from various online judges.
- Download multiple problems from a contest.
- Compile and test solutions, including custom and interactive judges.
- Generate test cases and reference answers.
- Submit solutions to judges.
- Watch submission results.
- List problems and contests you've downloaded.

Currently supported online judges:

- AtCoder
- AtCoder Problems (Virtual Contests)
- Yukicoder

cpg currently supports Linux. Building requires Rust 1.91 or newer.

## Documentation

See the [documentation site](https://sevenc-nanashi.github.io/competitive-programming-cli/)
for installation, configuration, and command usage. The Markdown sources are
available in [docs/content](docs/content/index.md).

To build the documentation locally on Linux, install Rust 1.91 or newer and the
tools in `mise.toml`, then run:

```bash
cd docs
aube ci
aube run build
```

`aube run build` and `aube run dev` regenerate the command reference from
`cargo run --locked -- __usage_spec__` using
[`usage generate markdown`](https://usage.jdx.dev/cli/reference/generate/markdown).
Edit the command definitions and help in `src/cli.rs`; the generated
`docs/cpg.usage.kdl` and `docs/content/command-reference.md` are ignored by Git.
Run `aube run reference` inside `docs` to regenerate only the reference.

## Releasing

For a new crate, publish its first version manually before configuring Trusted
Publishing.

Create the GitHub Actions environment `publish` and configure a
[crates.io Trusted Publisher](https://crates.io/docs/trusted-publishing) for
`competitive-programming-cli` with these settings:

| Setting           | Value                         |
| ----------------- | ----------------------------- |
| Repository owner  | `sevenc-nanashi`              |
| Repository name   | `competitive-programming-cli` |
| Workflow filename | `release.yml`                 |
| Environment       | `publish`                     |

Run the **Release** workflow manually from the branch to release, supplying a
version without the `v` prefix (for example, `0.1.0` or `0.1.0-rc.1`). The
environment's deployment rules must allow that branch.

After CI passes, the workflow updates `Cargo.toml` and `Cargo.lock` in a release
commit, verifies the package, builds the Linux binary, and publishes to crates.io
using a short-lived OIDC token. It then pushes the `v<version>` tag and creates a
GitHub release containing the binary archive.
Prerelease versions are marked as prereleases on GitHub. The version commit is
only pushed as a tag; the source branch keeps its existing version.

## Acknowledgements

This tools is heavily inspired by following tools:

- [online-judge-tools/oj](https://github.com/online-judge-tools/oj)
- [online-judge-tools/template-generator](https://github.com/online-judge-tools/template-generator)

```
MIT License

Copyright (c) 2020 Kimiyuki Onaka

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
