# Documentation

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
