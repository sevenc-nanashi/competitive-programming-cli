# Competitive Programming CLI (cpcli)

> [!WARNING]
> This project is in early development.
>
> - Configuration and metadata formats may change.
> - Only Linux is supported for now.

cpcli is a command-line interface tool for competitive programming.

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

cpcli currently supports Linux. Building requires Rust 1.91 or newer.

## Features

### Configuration

Configuration is stored within `$XDG_CONFIG_HOME/cpcli` (called `$config` in this document) by default.
Overridable with `$CPCLI_CONFIG_HOME` environment variable.
When XDG variables are unset, the configuration directory is `~/.config/cpcli`
and the data directory is `~/.local/share/cpcli`.

Run the interactive setup before downloading or listing problems:

```bash
cpcli init
```

It asks for the workspace root (default: `~/cpcli`), creates `config.toml`, the
workspace root, and the four template directories below, then prints a guide
to language settings, templates, login, and downloading problems. Relative paths
are saved as absolute paths. Re-running it keeps an existing configuration and
template files and creates any missing directories.

If configuration file of oj-prepare exists, it will be imported automatically.

You can also set the workspace root manually in `$config/config.toml`:

```toml
root = "/home/your-name/competitive-programming"
```

Language settings shown below belong in the same file. Path settings and CLI
path arguments expand a leading `~` or `~/` to `$HOME`, including `root`, source
files, test/generation directories, judge files, and configuration/Cookie paths.
For example, `root = "~/competitive-programming"` and
`cpcli test --test-dir "~/cases" "~/solutions/solve.cpp"` are supported.
Other environment variables in TOML paths are not expanded. Arguments after
`--` are passed directly to the command; use your shell's expansion when needed.

- `$config/config.toml`: Configuration file.
- `$config/workspace_template`: Template for workspace directory.
  - Files and directories within this directory is called "workspace template".
  - This should contain root files for your workspace, such as `Gemfile`, `Cargo.toml`, etc.
- `$config/problem_template`: Template for problem directory.
  - Files and directories within this directory is called "problem template".
  - This should contain files for a single problem, such as `solve.cpp`, `naive.rb`, etc.
- `$config/contest_template`: Template for contest directory.
  - Files and directories within this directory is called "contest template".
  - This should contain files for a contest, `Cargo.toml` with workspace configuration, etc.
- `$config/single_problem_template`: Template for single problem directory.
  - Files and directories within this directory is called "single problem template".
  - This should contain files that makes single problem directory self-contained, such as overwriting `Cargo.toml` with one without workspace dependencies, etc.

### Login

This tool does not provide login form itself.
You need to prepare cookies for each online judge and save them in Netscape HTTP Cookie Format.
For example, you can use [cookies.txt](https://addons.mozilla.org/ja/firefox/addon/cookies-txt/) Firefox extension to export cookies.

The cookies file should be saved in `$XDG_DATA_HOME/cpcli/cookies` by default.
Overridable with `$CPCLI_COOKIES_HOME` environment variable.
`login` verifies the session before saving `<service>.txt` with mode `600` in a
directory with mode `700`. An unsuccessful login leaves the previous cookies intact.
AtCoder Problems uses the AtCoder session; both `login atcoder` and
`login atcoder-problems` save `atcoder.txt`. Expired sessions require a fresh export.

```bash
# Login to an online judge
cpcli login atcoder --cookie-file /path/to/cookies.txt
```

### Download single problem

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
- Test cases (stored in `test/sample.in` and `test/sample.out`).

Multiple samples are named `sample-1.in`, `sample-1.out`, `sample-2.in`, etc.
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

### Download contest problems

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

### Open the current problem or contest

```bash
cpcli open
# Or shortcut
cpcli o
```

Opens the problem or contest URL from the nearest `.cpcli.toml` in your default
browser. This also works from subdirectories. A problem directory inside a
contest opens that problem; the contest directory opens the contest page.

### Test a solution

You can test a solution against the sample test cases.
This command will execute the command specified in the configuration file for the language of the solution file if single argument is given:

```toml
# Example configuration for C++
[language.cpp]
extensions = ["cpp"]
compile = "g++ -std=c++23 -Wall -Wextra -o {binary} {input}"
run = "{binary}"

[language.cpp.profile.fast]
compile = "g++ -std=c++23 -O2 -Wall -Wextra -o {binary} {input}"

# Example configuration for Ruby
[language.ruby]
extensions = ["rb"]
compile = "ruby -c {input}"
run = "ruby {input}"
```

Optional `language.<name>.preprocess` runs before compilation (or execution for
interpreted languages) and before submission. `language.<name>.presubmit` runs
only for submission, after `preprocess`. Each command receives the current
source on stdin and as the shell-quoted `{input}` path, runs in the source
directory, and must write the transformed UTF-8 source to stdout. A failed
command or empty output stops the operation.

For example, [ACL's expander](https://github.com/atcoder/ac-library/blob/master/expander.py)
can expand headers before both local compilation and submission:

```toml
[language.cpp]
extensions = ["cpp"]
preprocess = "python3 ~/ac-library/expander.py --console --lib ~/ac-library {input}"
compile = "g++ -std=c++23 -o {binary} {input}"
run = "{binary}"
```

Use `presubmit` instead of `preprocess` to apply that command only when
submitting. In a two-stage pipeline, `presubmit` receives the output of
`preprocess`. Transformations run once per source, preserve the original file,
and use temporary files with the same extension in the source directory so
relative includes keep working. The template checksum check runs before both
stages. Direct commands after `--` do not use language transformations.

```bash
# Test a solution against the sample test cases
cpcli test ./solution.cpp
# -> g++ -std=c++23 -Wall -Wextra -o solution ./solution.cpp && ./solution < ./test/sample.in

# Or shortcut
cpcli t ./solution.cpp

# Or specify the test case directory
cpcli test --test-dir ./random ./solution.cpp
# -> g++ -std=c++23 -Wall -Wextra -o solution ./solution.cpp && ./solution < ./random/sample.in

# Or specify the profile to use for compilation
cpcli test --profile fast ./solution.cpp
# -> g++ -std=c++23 -O2 -Wall -Wextra -o solution ./solution.cpp && ./solution < ./test/sample.in
```

Or you can specify the command to execute directly after `--`:

```bash
# Test a solution against the sample test cases with custom command
cpcli test -- ruby ./solution.rb
```

For TLE and MLE, you can use `--time-limit` and `--memory-limit` options to specify the time limit and memory limit for each test case.

```bash
# Test a solution against the sample test cases with time limit of 2000ms and memory limit of 256MB
cpcli test --time-limit 2000 --memory-limit 256 ./solution.cpp
```

The time limit measures wall-clock time. The memory limit is in MiB and uses
the solution process group's resident memory, sampled from `/proc` every 10 ms.
Short memory peaks may be missed; shared pages may be counted more than once.
Compilation runs once before testing and is outside these limits. Limits and
Ctrl-C terminate the process group, including children that inherit that group.

Each case reports `AC`, `WA`, `RE`, `TLE`, or `MLE`, elapsed time, and peak sampled
memory. The exit code is `0` when all cases pass, `1` when a case fails, `2` for
configuration/command errors, and `130` after interruption. Without
`--test-dir`, file-based tests read the source directory's `test` directory;
direct commands read `./test`. Every `.in` requires a matching `.out`.

For stripping trailing white-space in the output, you can use `--strip` option to ignore trailing white-space differences between the expected output and the actual output.

```bash
# Test a solution against the sample test cases with stripping trailing white-space
cpcli test --strip ./solution.cpp
```

For CRLF/LF insensitive comparison, you can use `--ignore-line-ending` option to ignore line ending differences between the expected output and the actual output.
This is enabled by default, but you can disable it with `--no-ignore-line-ending` option.

```bash
# Test a solution against the sample test cases with CRLF/LF insensitive comparison
cpcli test --ignore-line-ending ./solution.cpp
```

For fast failure, you can use `--fast-fail` option to stop testing after the first failed test case.

```bash
# Test a solution against the sample test cases with fast failure
cpcli test --fast-fail ./solution.cpp
```

For floating point comparison, you can use `--float-error` option to specify the acceptable error for floating point comparison.
It will allow if the absolute difference or relative difference between the expected output and the actual output is less than or equal to the specified error.

```bash
# Test a solution against the sample test cases with floating point comparison
cpcli test --float-error 1e-6 ./solution.cpp

# Or allow absolute error only
cpcli test --float-error 1e-6 --float-error-type absolute ./solution.cpp

# Or allow relative error only
cpcli test --float-error 1e-6 --float-error-type relative ./solution.cpp
```

For custom judges, you can specify the command to execute for each test case:
The judge will receive three arguments: the test input file (`{test_input}`), the expected output file (`{test_output}`), and the actual output file from the solution (`{solution_output}`).
If the judge command does not have these placeholders, cpcli will append them to the end of the command.
Judges should return exit code 0 for accepted, otherwise return non-zero exit code for rejected.

```bash
# Test a solution against the sample test cases with custom judge
cpcli test --judge ./judge.rb ./solution.rb

# Or directly specify the command to execute for each test case
cpcli test --judge "ruby ./judge.rb {test_input} {test_output} {solution_output}" ./solution.rb
```

### Test interactive problems

You can test interactive problems with custom judge.
The judge's standard input will receive the output from the solution, and the judge's standard output will be sent to the solution's standard input.
cpcli will prefix `?` for the judge's output and `!` for the solution's output.
If test files exist, the judge will receive the path as `{test_input}` and `{test_output}` arguments, and cpcli will run the judge and solution for each test case.
Unlike other test commands, this command can be run without test files, and the judge will be run only once with no test files.

```bash
# Test an interactive problem with custom judge
cpcli test --interactive --judge ./judge.rb ./solution.rb
```

### Randomly generate test cases

You can randomly generate test cases using a generator script.

```bash
# Specify generator
cpcli generate ./random.rb

# Or shortcut
cpcli g ./random.rb

# Or specify directory to save the generated test cases
cpcli generate --dir ./random ./random.rb

# Or specify full command to execute
cpcli generate -- ruby ./random.rb
```

100 inputs are generated by default; `--count 10` generates ten inputs. The
default directory is `./test`. Files are named `random-0001.in`, incrementing
past existing `.in` or `.out` files. Existing cases are preserved and failed
generator runs do not leave partial output files.

After generating test cases, you can run naive solution against the generated test cases to verify the correctness of the solution.

```bash
# Run naive solution against the generated test cases
cpcli generate --answer ./naive.rb

# Or specify full command to execute
cpcli generate --answer -- ruby ./naive.rb
```

`--answer` processes every `.in` without a corresponding `.out`, preserving
existing answers. It cannot be combined with `--count`.

### Submit solution

You can submit a solution to a problem.

```bash
# Submit a solution to a problem
cpcli submit ./solution.cpp

# Or shortcut
cpcli s ./solution.cpp
```

The problem is detected using the nearest `.cpcli.toml` in the solution file's
directory or its ancestors.
You can also specify the problem URL using `--problem`.

```bash
# Submit a solution to a problem in another directory
cpcli submit ./solution.cpp --problem https://atcoder.jp/contests/abc473/tasks/abc473_f
```

Configure the judge's language ID for each file type, or pass `--language ID`:

```toml
[language.cpp.submit]
atcoder = "<AtCoder language ID>"
yukicoder = "<yukicoder language ID>"
```

cpcli fetches the available languages and displays their IDs if the configured
ID is missing or invalid. AtCoder Problems uses the `atcoder` language setting.
A successful submission prints its ID and URL. If the judge's response is
ambiguous, cpcli reports that the outcome is unknown; check `results` before
submitting again.

`submit` warns and stops if the source is identical to its saved template, including
when `--problem` is specified. To intentionally submit the unchanged file:

```bash
cpcli submit ./solution.cpp --allow-submit-unchanged-solution
```

### List results of submissions

You can list the results of your submissions.

```bash
# List results of submissions
cpcli results

# Or shortcut
cpcli r

# Or monitor the results in an interactive terminal UI
cpcli results --ui
```

The nearest `.cpcli.toml` in the current directory or its ancestors selects the
problem or contest. Results include your browser submissions as well as cpcli
submissions. The newest 20 are shown by default; use `--limit N` to change this.
On a terminal, both modes use aligned columns and colored status labels. URLs
appear below each submission in the normal listing. `--no-color` or `NO_COLOR`
disables colors.

When stdout is piped or redirected, both `results` and `results --ui` print a
single snapshot in the original tab-separated format, with the same header and
column order and no colors or terminal controls.

### List downloaded directories

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

## Installation

To install the current checkout:

```bash
cargo install --path . --locked
```

After a crates.io release is published, you can install cpcli using cargo:

```bash
# Build from source
cargo install competitive-programming-cli

# Or download the pre-built binaries using cargo-binstall
cargo binstall competitive-programming-cli
```

Tagged releases provide `cpcli-x86_64-unknown-linux-gnu.tar.gz` and a SHA-256
checksum on the [releases page](https://github.com/sevenc-nanashi/competitive-programming-cli/releases).
You can install those binaries manually or using package managers like `mise`.

```bash
# Using mise
mise use -g github:sevenc-nanashi/competitive-programming-cli
```

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
