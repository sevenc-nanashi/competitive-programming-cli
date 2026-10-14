# Workspaces

## Download sample cases

`cpg download` (`cpg d`) downloads one problem's samples into `./test/`.
It does not require a workspace root or copy templates, run setup commands, or write metadata.
Use `-d` / `--directory` to choose another sample directory. Existing sample files
are never overwritten; unrelated files in the directory are preserved.

```bash
cpg d https://atcoder.jp/contests/abc473/tasks/abc473_f
cpg d https://atcoder.jp/contests/abc473/tasks/abc473_f -d samples
```

## Workspace root

The directory you downloaded a single problem or contest problems will be called a "workspace".

Workspaces will be located under the workspace root, which is configured in `~/.config/cpg/config.toml`.
For example, if the workspace root is `~/competitive-programming`, a single problem
downloaded from AtCoder will be located at `~/competitive-programming/atcoder/problems/abc473_f`, and a
contest will be located at `~/competitive-programming/atcoder/contests/abc473`.
This feature is heavily inspired by [ghq](https://github.com/x-motemen/ghq), a git repository management tool
which fixes the directory structure for git repositories. (`$root/$host/$path`, like `~/ghq/github.com/sevenc-nanashi/competitive-programming-cli`)

After using cpg for a while, your workspace root will look like this:

```file-tree
- ~/cpg/
  - atcoder/
    - contests/
      - aalc001/
      - abc465/
      - abc466/
      - abc467/
      - .../
      - adt_all_20260908_1/
      - adt_all_20260909_1/
      - .../
    - problems/
      - abc294_g/
      - abc313_d/
      - .../
  - yukicoder/
    - contests/
      - 594/
      - .../
  - share-oj/
    - contests/
      - dc6fc9dd-4019-463a-a486-12c3fc10af59/
    - problems/
      - 898f6783-900e-4035-9a03-500571a20d06/
      - 796156e7-bc25-4b68-82ec-494135b75dc5/
      - .../
  - .../
```

## Prepare a single problem

You can download a single problem from a contest.
The directory will contain:

- `.cpg.toml` file, which contains metadata of the problem.
- Workspace template files.
- Problem template files.
- Single problem template files.
- Test cases (stored in `test/sample-1.in` and `test/sample-1.out`).

Samples are always numbered starting at 1, even when there is only one.
Additional samples are named `sample-2.in`, `sample-2.out`, etc.
The `.cpg.toml` metadata records the service, problem ID, canonical URL, title,
and original contest/internal IDs where applicable.

If multiple template files contain the same file name, the last one will overwrite the previous ones.
Existing problem or contest directories are never overwritten. Downloads are
staged in a temporary directory and published only after all files are ready.
Template symlinks are rejected.
Optional [`[setup]` commands](./configuration.md#commands-after-copying-templates)
run after templates are copied and must succeed before the workspace is published.

```bash
# Download a problem from AtCoder
cpg prepare https://atcoder.jp/contests/abc473/tasks/abc473_f

# Or shortcut
cpg p https://atcoder.jp/contests/abc473/tasks/abc473_f
```

There are no template engines or variable substitutions. The templates are copied as-is.
Use the `[setup]` section in the configuration file to run commands after copying templates,
like replacing `package.name` in `Cargo.toml` with the problem ID, or creating a `Makefile` from a template.

Your single problem directory will look like this:

```file-tree
- abc473_f/
  - .cpg.toml
  - ac-library/ **
  - test/
    - sample-1.in
    - sample-1.out
    - sample-2.in
    - sample-2.out
    - sample-3.in
    - sample-3.out
    - sample-4.in
    - sample-4.out
  - Gemfile **
  - Gemfile.lock **
  - compile_flags.txt **
  - judge.rb **
  - main.cpp **
  - main.rb **
  - naive.rb **
```

## Prepare contest problems

You can download all problems from a contest.
The contest directory will contain:

- `.cpg.toml` file, which contains metadata of the contest.
- Workspace template files.
- Contest template files.

The problem directories will contain:

- Problem template files.
- Problem metadata and sample test cases.
- (Note that this does not contain workspace template files)

Contest metadata retains the ordered problem list.

Similar to single problem downloads, optional [`[setup]` commands](./configuration.md#commands-after-copying-templates)
run after templates are copied and must succeed before the workspace is published.

```bash
# Download contest problems from AtCoder
cpg prepare https://atcoder.jp/contests/abc473

# Or shortcut
cpg p https://atcoder.jp/contests/abc473
```

Your contest directory will look like this:

```file-tree
- abc479/
  - .cpg.toml
  - ac-library/ **
  - 1_abc479_a/
    - .cpg.toml
    - test/
      - sample-1.in
      - sample-1.out
      - sample-2.in
      - sample-2.out
      - sample-3.in
      - sample-3.out
    - main.cpp **
    - main.rb **
    - naive.rb **
    - judge.rb **
  - 2_abc479_b/
  - 3_abc479_c/
  - 4_abc479_d/
  - 5_abc479_e/
  - 6_abc479_f/
  - 7_abc479_g/
  - Gemfile **
  - Gemfile.lock **
  - compile_flags.txt **
```

## Open the current problem or contest

```bash
cpg open
# Or shortcut
cpg o
```

Opens the problem or contest URL from the nearest `.cpg.toml` in your default
browser. This also works from subdirectories. A problem directory inside a
contest opens that problem; the contest directory opens the contest page.

Use `--url-only` to print just the URL to stdout without opening a browser:

```bash
cpg open --url-only
# Or shortcut
cpg o -n
```

Or use `--clipboard` to copy the URL to the clipboard instead of opening a browser:

```bash
cpg open --clipboard
# Or shortcut
cpg o -c
```

## List workspaces

You can list the workspaces you've downloaded using the `list` command.

By default, `list` shows workspace directories: contests and single problems.
Paths are relative to the workspace root, which you can get with `cpg config --root`.
Choose one of the following mutually exclusive filters:

| Option                  | Directories listed                              |
| ----------------------- | ----------------------------------------------- |
| `--workspace` (default) | Contests and single problems                    |
| `--contests`            | Contests                                        |
| `--problems`            | Single problems                                 |
| `--all-problems`        | Single problems and individual contest problems |

```bash
# List workspaces you've downloaded
cpg list

# Show the workspace root
cpg config --root

# List contest directories
cpg list --contests

# List single problem directories
cpg list --problems

# Include individual problems within contests
cpg list --all-problems
```

Use `--format` (`-f`) to customize each output line.

| Placeholder | Description                           |
| ----------- | ------------------------------------- |
| `{path}`    | Relative path from the workspace root |
| `{service}` | Online judge service name             |
| `{id}`      | Problem or contest ID                 |
| `{title}`   | Problem or contest title              |
| `{url}`     | Problem or contest URL                |
| `\t`        | Tab character                         |

```bash
cpg list --format '{path}\t{path} ({title})'
```

Use `--delimiter` to choose the separator appended after each record:
`line` (the default newline), `null` (NUL), `tab`, or any single character.
For example, pass NUL-separated records to `fzf --read0`:

```bash
cpg list -f '{path}\t{path} ({title})' --delimiter null |
    fzf --read0 --delimiter=$'\t' --with-nth=2..
```

This command is for piping the output to other commands, such as `fzf`.

If you need more advanced formatting, you can use `--json` to output JSON for each record
and parse it with `jq` or other JSON tools.

For example, you can create `ccd` command which changes the current working directory to the selected problem's directory.

```bash
ccd() {
    local root dir
    root="$(CPG_LOG=none cpg config --root)" || return
    dir="$(CPG_LOG=none cpg list -f '{path}\t{path} ({title})' | fzf --delimiter=$'\t' --with-nth=2..)" || return
    dir="${dir%%$'\t'*}"
    [ -n "$dir" ] && cd -- "$root/$dir"
}
```
