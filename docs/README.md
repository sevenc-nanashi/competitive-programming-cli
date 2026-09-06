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

To update the demo GIF and asciinema recording, run `mise run demo` from the
repository root. Recording requires Ruby, FFmpeg, and the tools in `mise.toml`.
Edit `demo/demo.tape` to change the commands and timing. The recording uses local
mock data and a temporary workspace. Mock submissions start as `WJ`; fetching
results at least a few seconds after submission runs the sample tests and caches the
verdict and maximum runtime. The mock judge uses the commands in
`mock_service/service.toml`, with a two-second limit per sample and a 30-second
compilation limit.

Replay the recording with `asciinema play docs/public/demo.cast` from the
repository root.

MDX pages can embed it with the globally registered component in
`components/Asciinema.tsx`. `poster` selects an optional preview time:

```mdx
<Asciinema src="/competitive-programming-cli/demo.cast" poster="npt:11" />
```
