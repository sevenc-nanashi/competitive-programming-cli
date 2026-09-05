# Workspaces

The directory you downloaded a single problem or contest problems will be called a "workspace".

## Download single problem

You can download a single problem from a contest.
The downloaded problem will be saved in a directory `$root/$host/problems/$problem_id`, where:

- `$root` is the root directory of your workspace, which is specified in the configuration file.
- `$host` is the host of the online judge, such as `atcoder`.
- `$problem_id` is the problem ID, such as `abc473_f`.

The directory will contain:

- `.cpcli.toml` file, which contains metadata of the problem.
- Workspace template files.
- Problem template files.
- Single problem template files.
- Test cases (stored in `test/sample-1.in` and `test/sample-1.out`).

Samples are always numbered starting at 1, even when there is only one.
Additional samples are named `sample-2.in`, `sample-2.out`, etc.
The `.cpcli.toml` metadata records the service, problem ID, canonical URL, title,
and original contest/internal IDs where applicable.

If multiple template files contain the same file name, the last one will overwrite the previous ones.
Existing problem or contest directories are never overwritten. Downloads are
staged in a temporary directory and published only after all files are ready.
Template symlinks are rejected.

```bash
# Download a problem from AtCoder
cpcli download https://atcoder.jp/contests/abc473/tasks/abc473_f

# Or shortcut
cpcli d https://atcoder.jp/contests/abc473/tasks/abc473_f
```

## Download contest problems

You can download all problems from a contest.
The downloaded problems will be saved in a directory `$root/$host/contests/$contest_id/$index_$problem_id`, where:

- `$root` is the root directory of your workspace, which is specified in the configuration file.
- `$host` is the host of the online judge, such as `atcoder`.
- `$contest_id` is the contest ID, such as `abc473`.
- `$index` is the 1-indexed and zero-padded index of the problem in the contest, such as `01`.
  - This exists for sorting problems in the order they appear in the contest, for when:
    - The contest contains multiple problems from different contests (e.g. AtCoder Daily Training),
    - The problem IDs are not sorted in the order they appear in the contest (e.g. Yukicoder).
- `$problem_id` is the problem ID, such as `abc473_a`.

The contest directory will contain:

- `.cpcli.toml` file, which contains metadata of the contest.
- Workspace template files.
- Contest template files.

The problem directories will contain:

- Problem template files.
- Problem metadata and sample test cases.
- (Note that this does not contain workspace template files)

Contest metadata retains the ordered problem list. AtCoder Problems virtual
contests preserve their configured order and refer back to the original AtCoder
problems; yukicoder uses the contest's problem ID list.

```bash
# Download contest problems from AtCoder
cpcli prepare https://atcoder.jp/contests/abc473

# Or shortcut
cpcli p https://atcoder.jp/contests/abc473
```

## Open the current problem or contest

```bash
cpcli open
# Or shortcut
cpcli o
```

Opens the problem or contest URL from the nearest `.cpcli.toml` in your default
browser. This also works from subdirectories. A problem directory inside a
contest opens that problem; the contest directory opens the contest page.

## List workspaces

You can list the workspaces you've downloaded using the `list` command.

By default, `list` shows workspace directories: contests and standalone problems.
Choose one of the following mutually exclusive filters:

| Option                  | Directories listed                                  |
| ----------------------- | --------------------------------------------------- |
| `--workspace` (default) | Contests and standalone problems                    |
| `--contests`            | Contests                                            |
| `--problems`            | Standalone problems                                 |
| `--all-problems`        | Standalone problems and individual contest problems |

```bash
# List workspaces you've downloaded
cpcli list

# List workspaces with absolute paths
cpcli list --path

# List contest directories
cpcli list --contests

# List standalone problem directories
cpcli list --problems

# Include individual problems within contests
cpcli list --all-problems
```

This command is for piping the output to other commands, such as `fzf`.
For example, you can create `ccd` command which changes the current working directory to the selected problem's directory.
This feature is heavily inspired by [ghq](https://github.com/x-motemen/ghq).

```bash
ccd() {
    dir="$(cpcli list --path | fzf)"
	[ -n "$dir" ] && cd "$dir"
}
```
