# Configuration and login

## Configuration

Configuration is stored within `$XDG_CONFIG_HOME/cpg` (called `$config` in this document) by default.
Overridable with `$CPG_CONFIG_HOME` environment variable.
When XDG variables are unset, the configuration directory is `~/.config/cpg`
and the data directory is `~/.local/share/cpg`.

Run the interactive setup before downloading or listing problems:

```bash
cpg init
```

It creates configuration files and directories. If configuration file of
oj-prepare exists, it will be imported automatically.

You can also set the workspace root manually in `$config/config.toml`:

```toml
root = "/home/your-name/competitive-programming"
```

The JSON Schema for the configuration file is available on
`https://raw.githubusercontent.com/sevenc-nanashi/competitive-programming-cli/refs/tags/v{version}/docs/public/config.schema.json`,
or print it locally with `cpg config --schema`.

In addition to the configuration file, the following directories are created:

- `$config/config.toml`: Configuration file.
- `$config/config.local.toml`: Optional machine-specific overrides for `config.toml`.
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

These directories can be obtained by running `cpg config`:

```bash
cpg config
# Workspace root: /home/your-name/competitive-programming
# Configuration directory: /home/your-name/.config/cpg
# Cookies directory: /home/your-name/.local/share/cpg/cookies
# oj virtual environment directory: /home/your-name/.local/share/cpg/oj-venv
# Workspace template directory: /home/your-name/.config/cpg/workspace_template
# Problem template directory: /home/your-name/.config/cpg/problem_template
# Contest template directory: /home/your-name/.config/cpg/contest_template
# Single problem template directory: /home/your-name/.config/cpg/single_problem_template

# Print the workspace root
cpg config --root

# Print the configuration directory
cpg config --config-dir
cpg config --cookies-dir
cpg config --oj-venv-dir
cpg config --workspace-template-dir
cpg config --problem-template-dir
cpg config --contest-template-dir
cpg config --single-problem-template-dir
```

## Local overrides

Create `config.local.toml` next to `config.toml` to override settings for the
current machine. Both files are read from `$CPG_CONFIG_HOME` when it is set,
otherwise from the configuration directory described above.

```toml
# config.local.toml
root = "~/local-competitive-programming"

[language.cpp]
compile = "clang++ -std=c++23 -O2 -o {binary} {input}"

[clipboard]
kind = "command"
command = "/mnt/c/Windows/System32/clip.exe"
```

## Logging

`CPG_LOG` accepts [tracing-subscriber](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html)'s
`EnvFilter` syntax. Logs never appear on stdout.

```bash
CPG_LOG=debug cpg prepare https://atcoder.jp/contests/abc473/tasks/abc473_f
CPG_LOG=info,cpg=debug,reqwest=warn cpg prepare https://atcoder.jp/contests/abc473/tasks/abc473_f
CPG_LOG=off cpg list
```

## Directory prefixes

To use lowercase letters for contest problem directory prefixes, add this at the
top level of `config.toml` (or `config.local.toml`):

```toml
alphabetic = true
```

Prefixes are `a`, ..., `z`, `aa`, ... and are left-padded with underscores to
match the longest prefix. For 27 problems, directories start with `_a_`, ..., `_z_`,
`aa_`. The default is `false`, which keeps zero-padded numeric prefixes.

## Clipboard

`cpg submit --clipboard` copies the source after preprocessing and presubmit.
Choose its backend with `[clipboard]` in `$config/config.toml`:

```toml
[clipboard]
kind = "osc52"
```

`osc52` is the default when the section is omitted. It sends an OSC 52 escape
sequence to stderr, asking the terminal to copy the text. The terminal must
support OSC 52 and allow clipboard writes, and the stderr stream must be a
terminal.

By default, clipboard contents are cleared when `cpg submit --clipboard` fails.
This is to prevent accidentally submitting the wrong source, because you will
paste the empty clipboard into the submission form, and many online judges reject
empty submissions. You can disable this behavior with:

```toml
[clipboard]
clear_on_failure = false
```

To use the system clipboard through [arboard](https://docs.rs/arboard/latest/arboard/):

```toml
[clipboard]
kind = "arboard"
```

On Linux, a clipboard manager must retain the text after cpg exits; otherwise,
use a command such as `wl-copy` or `xclip` below.

To use a clipboard command instead:

```toml
[clipboard]
kind = "command"
command = "wl-copy"
```

`command` is required for this backend and runs through `sh -c` (`cmd /C` on Windows) in the current
directory. The complete UTF-8 text is piped to its standard input without adding
a newline. Command stdout and stderr go to cpg's stderr. A nonzero exit status
fails the copy; Ctrl-C stops the command and its process group.
For X11, you can use `command = "xclip -selection clipboard"`.

On WSL, you might want to use `command = "/mnt/c/Windows/System32/clip.exe"` to
copy the content to the Windows clipboard, in case arboard fails to work.

## Remote mode

cpg has a remote mode that changes behavior to work on a remote server.

- Clipboard backend is set to `osc52`.
- Copies the URL instead of opening it in a browser when `--open` is specified.

By default, remote mode is automatically enabled when:

- SSH environment variables are detected,
- `CPG_REMOTE` is set to `1`, or
- `--remote` is specified on the command line.

You can disable SSH detection in `$config/config.toml` with:

```toml
remote = "manual"
```

## Commands after copying templates

Add `[setup]` to `$config/config.toml` to run shell commands after copying templates:

```toml
[setup]
workspace = ["git init", "bundle install"]
problem = "ruby setup_problem.rb"
contest = "ruby setup_contest.rb"
single_problem = "ruby setup_single_problem.rb"
```

Each key accepts a single command string or an array of command strings, which
run in order. Put scripts in the corresponding template directory so
they are available when the commands run. Commands also run when that template
directory is absent, allowing setup entirely through commands.

| Key              | Template                  | Working directory              |
| ---------------- | ------------------------- | ------------------------------ |
| `workspace`      | `workspace_template`      | Single problem or contest root |
| `problem`        | `problem_template`        | Each problem directory         |
| `contest`        | `contest_template`        | Contest root                   |
| `single_problem` | `single_problem_template` | Single problem root            |

<!-- For `cpg prepare` with a problem URL, templates are copied in the order: -->
<!-- `workspace`, `problem`, then `single_problem`. Setup commands run in the order -->
<!-- `problem`, `workspace`, then `single_problem`. -->
<!-- For a contest URL, `workspace` and `contest` templates are copied at the contest -->
<!-- root before the `problem` template is copied into each problem directory. -->
<!-- Each problem's setup runs there; after all problems are prepared, `workspace` -->
<!-- setup runs, followed by `contest` setup at the contest root. -->

Later templates overwrite earlier files with the same path.

Template and setup commands are applied in this order:

- Write `.cpg.toml` with problem or contest metadata.
- Copy `workspace_template` to the workspace root.
- Copy `contest_template` to the contest root. (for contests)
- Copy `problem_template` to each problem directory.
- Copy `single_problem_template` to the problem root. (for single problems)
- Run `[setup.problem]` in each problem directory.
- Run `[setup.workspace]` in the workspace root.
- Run `[setup.contest]` in the contest root. (for contests)
- Run `[setup.single_problem]` in the problem root. (for single problems)

You would want to set `bundle install` or `git clone https://github.com/atcoder/ac-library.git` in the workspace template,
for example.

## Login

This tool does not provide login form itself.
You need to prepare cookies for each online judge and save them in Netscape HTTP Cookie Format.
For example, you can use [Get cookies.txt LOCALLY](https://github.com/kairi003/Get-cookies.txt-LOCALLY) extension.

The cookies file is saved in `$XDG_DATA_HOME/cpg/cookies` by default, and the permission is `600`.

```bash
# Login to an online judge
cpg login atcoder --cookie-file /path/to/cookies.txt
```

Use `--info` to verify the saved session and print its username and profile URL:

```bash
cpg login atcoder --info
cpg login yukicoder --info
cpg login codeforces --info
cpg login share-oj --info
cpg login atcoder-problems --info
```

It prints the user name on the first stdout line and the profile URL on the second.

## Experimental oj support

URLs outside the native AtCoder, AtCoder Problems, yukicoder, and Codeforces backends are
automatically passed to a [fork of online-judge-api-client](https://github.com/sevenc-nanashi/online-judge-tools-api-client).
Supported URLs and operations depend on that client and the judge. For example:

```bash
cpg download https://onlinejudge.u-aizu.ac.jp/problems/ITP1_1_A
```

Install `uv` and make it available on `PATH`. On first use, cpg creates
`$XDG_DATA_HOME/cpg/oj-venv`, defaulting to `~/.local/share/cpg/oj-venv`, and
installs its pinned API client.

oj backends use `oj+<host>` as the service name. For example, Aizu Online Judge is `oj+onlinejudge.u-aizu.ac.jp`.

```bash
cpg login oj+onlinejudge.u-aizu.ac.jp --cookie-file /path/to/cookies.txt
```

This backend is experimental. Login verification and submission may fail or be
unsupported for a judge, and `cpg results` is not supported. Downloading a contest
can be slower than with native backends.

## Language settings

Add `[language.<name>]` tables to `$config/config.toml`. cpg selects a language
by the source file's extension. These settings apply to solutions, generators, reference solutions, and judge files.

| Key          | Required | Purpose                                                                              |
| ------------ | -------- | ------------------------------------------------------------------------------------ |
| `extensions` | Yes      | File extensions without the leading dot, such as `["cpp", "cc"]`.                    |
| `run`        | Yes      | Shell command to execute the solution or script.                                     |
| `compile`    | No       | Shell command to compile or check the source before running it.                      |
| `preprocess` | No       | Transform the source before local execution and submission.                          |
| `presubmit`  | No       | Transform the source only for submission, after `preprocess`.                        |
| `profile`    | No       | Named overrides for commands and submission language IDs, selected with `--profile`. |
| `submit`     | No       | Submission language IDs keyed by service, such as `atcoder`, `yukicoder`, and so on. |

In `compile` and `run`, `{input}` expands to the source path and `{binary}` to the same path with
its final extension removed on Linux/macOS or replaced with `.exe` on Windows.
Placeholder are automatically shell-quoted, so leave them unquoted.

When preprocessing is configured, `{input}` points to the transformed source.
Direct commands after `--` do not use these language settings.

All language commands (`compile`, `run`, `preprocess`, and `presubmit`), including
profile overrides, also support `{workspace}` and `{problem}`:

- `{workspace}`: the absolute path to the contest root or single problem directory.
- `{problem}`: the absolute path to the problem directory, even for sources in subdirectories.

```toml
[language.cpp]
extensions = ["cpp"]
compile = "g++ -std=c++23 -I{workspace}/ac-library -I{problem}/include -o {binary} {input}"
run = "{binary}"
```

### Build profiles

Use `[language.<name>.profile.<profile>]` to override these commands.
Note that this overrides the entire command, not appending extra commands or flags.
You can use an empty string (`""`) or `false` to disable an inherited command.

```toml
[language.python.profile.plain]
preprocess = false
presubmit = ""
compile = false
```

For example, use a different interpreter and submission language together:

```toml
[language.python.profile.pypy]
run = "pypy3 {input}"

[language.python.profile.pypy.submit]
yukicoder = "pypy3"
```

Submission runs only `preprocess` and `presubmit`; it does not compile or run the
source. An explicit `--language ID` takes precedence over the profile's `submit` setting.

### Executable files

If no configured extension matches, executable files use the built-in
`executable` language, which runs `{input}` without compilation. This works
with extensionless binaries and scripts with execute permission and a shebang
on Linux/macOS, and `.exe` or `.com` files on Windows:

```bash
cpg test ./a.out
cpg generate --count 10 ./generator
```

To customize the fallback, define `[language.executable]` in `config.toml`:

```toml
[language.executable]
extensions = []
run = "{input}"

[language.executable.profile.debug]
run = "env DEBUG=1 {input}"
```

### Source transformations

Optional `language.<name>.preprocess` runs before compilation (or execution for
interpreted languages) and before submission. `language.<name>.presubmit` runs
only for submission, after `preprocess`.

```toml
[language.cpp]
extensions = ["cpp"]
preprocess = "ruby ~/expand.rb {input} {processed}"
compile = "g++ -std=c++23 -O2 -o {binary} {input}"
run = "{binary}"
```

The same placeholder is available in `presubmit`. Use `presubmit` instead of
`preprocess` to apply a transformation only when submitting.
See the [ACL recipe](#expand-atcoder-library-headers) for an example.

### Submission language IDs

Add the judge's language ID to an existing language's `submit` table. For C++23:

```toml
[language.cpp.submit]
atcoder = "6017"
yukicoder = "cpp23"
```

IDs must be quoted strings, including numeric IDs. These IDs can be found when you
attempt to submit a solution with an unconfigured language with `cpg submit`.
A profile's `submit` table can also override these IDs.
See [submitting solutions](./submissions.md).

## Recipes

Copy the settings you need into `$config/config.toml` and install the compilers
or interpreters used by their commands.

### C++ with debugging and a fast profile

This configuration uses GCC with C++23. The default build enables debug symbols and
[UndefinedBehaviorSanitizer](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html).
`-fno-sanitize-recover=all` makes a detected error stop the program so the test
reports `RE`. The `fast` profile enables optimization and defines `ONLINE_JUDGE`.

```toml
[language.cpp]
extensions = ["cpp", "cc"]
compile = "g++ -std=c++23 -O0 -g -Wall -Wextra -fsanitize=undefined -fno-sanitize-recover=all -o {binary} {input}"
run = "{binary}"

[language.cpp.profile.fast]
compile = "g++ -std=c++23 -O2 -Wall -Wextra -DONLINE_JUDGE -o {binary} {input}"

[language.cpp.submit]
atcoder = "6017" # C++23 (GCC 15.2.0)
yukicoder = "cpp23"
# If you prefer Clang:
#
# atcoder = "6116" # C++23 (Clang 21.1.0)
# yukicoder = "cpp-clang"
#
# But note that Clang on yukicoder is C++17, so you may need to change -std=c++23 to -std=c++17 in the compile commands.
```

### Ruby

The compile command checks syntax with `ruby -c`. The run command executes the script.

```toml
[language.ruby]
extensions = ["rb"]
compile = "ruby -c {input}"
run = "ruby {input}"

[language.ruby.submit]
atcoder = "6087" # Ruby 3.4 (ruby 3.4.5)
yukicoder = "ruby"
```

### Python

The run command executes the script with Python 3.

```toml
[language.python]
extensions = ["py"]
run = "python3 {input}"

[language.python.submit]
atcoder = "6083" # PyPy 3.11-v7.3.20
yukicoder = "pypy3"

# Or if you prefer CPython:
#
# atcoder = "6082" # Python (CPython 3.13.7)
# yukicoder = "python3"
```

### Expand AtCoder Library headers

With [ac-library](https://github.com/atcoder/ac-library) checked out at
`~/ac-library`, use its [expander](https://github.com/atcoder/ac-library/blob/master/expander.py)
to inline ACL headers before local compilation and submission. This requires
Python 3 and GCC.

```toml
[language.cpp]
extensions = ["cpp"]
preprocess = "python3 ~/ac-library/expander.py --console --lib ~/ac-library {input} > {processed}"
compile = "g++ -std=c++23 -O2 -o {binary} {input}"
run = "{binary}"

[language.cpp.submit]
atcoder = "6017" # C++23 (GCC 15.2.0)
yukicoder = "cpp23"
```

To expand only for submission, rename `preprocess` to `presubmit` and add
`-I "$HOME/ac-library"` to your compilation command so local tests can find
the headers without expansion. Configure the appropriate
[submission language ID](#submission-language-ids) before submitting.
