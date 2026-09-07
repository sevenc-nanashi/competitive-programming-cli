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

It asks for the workspace root (default: `~/cpg`), creates `config.toml`, the
workspace root, and the four template directories below, then prints a guide
to language settings, templates, login, and downloading problems. Relative paths
are saved as absolute paths. Re-running it keeps an existing configuration and
template files and creates any missing directories.

If configuration file of oj-prepare exists, it will be imported automatically.

You can also set the workspace root manually in `$config/config.toml`:

```toml
root = "/home/your-name/competitive-programming"
```

The JSON Schema for the configuration file is available on
`https://raw.githubusercontent.com/sevenc-nanashi/competitive-programming-cli/refs/tags/v{version}/docs/public/config.schema.json`,
or print it locally with `cpg config --schema`.

After setting the root, inspect the current paths with:

```bash
cpg config
# Workspace root: /home/your-name/competitive-programming
# Configuration directory: /home/your-name/.config/cpg
# Cookies directory: /home/your-name/.local/share/cpg/cookies
# Workspace template directory: /home/your-name/.config/cpg/workspace_template
# Problem template directory: /home/your-name/.config/cpg/problem_template
# Contest template directory: /home/your-name/.config/cpg/contest_template
# Single problem template directory: /home/your-name/.config/cpg/single_problem_template

# Print only the workspace root, for use in scripts
cpg config --root

# Or print only one of the other directories
cpg config --config-dir
cpg config --cookies-dir
cpg config --workspace-template-dir
cpg config --problem-template-dir
cpg config --contest-template-dir
cpg config --single-problem-template-dir
```

Paths are absolute and reflect the environment overrides above and in
[Login](#login). These commands do not create directories or modify settings.
The flags are mutually exclusive. The full display and `--root` require a
configured root; run `cpg init` or set `root` first. All directory flags also work
before initialization. Template directories are located within the configuration
directory, including when `$CPG_CONFIG_HOME` is set.

[Language settings](#language-settings) belong in the same file. Path settings and CLI
path arguments expand a leading `~` or `~/` to `$HOME`, including `root`, source
files, test/generation directories, judge files, and configuration/Cookie paths.
For example, `root = "~/competitive-programming"` is supported.
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

## Clipboard

`cpg submit --clipboard` copies the source after preprocessing and presubmit.
Choose its backend with `[clipboard]` in `$config/config.toml`:

```toml
[clipboard]
kind = "arboard"
```

`arboard` is the default when the section is omitted. It uses the system
clipboard through [arboard](https://docs.rs/arboard/latest/arboard/).
On Linux, a clipboard manager must retain the text after cpg exits; otherwise,
use a command such as `wl-copy` or `xclip` below.

To use a clipboard command instead:

```toml
[clipboard]
kind = "command"
command = "wl-copy"
```

`command` is required for this backend and runs through `sh -c` in the current
directory. The complete UTF-8 text is piped to its standard input without adding
a newline. Command stdout and stderr go to cpg's stderr. A nonzero exit status
fails the copy; Ctrl-C stops the command and its process group.
For X11, you can use `command = "xclip -selection clipboard"`.

On WSL, you might want to use `command = "/mnt/c/Windows/System32/clip.exe"` to,
copy the content to the Windows clipboard, in case arboard fails to work.

## Commands after copying templates

Add `[setup]` to `$config/config.toml` to run shell commands immediately after
each corresponding template is copied:

```toml
[setup]
workspace = ["git init", "bundle install"]
problem = "ruby setup_problem.rb"
contest = "ruby setup_contest.rb"
single_problem = "ruby setup_single_problem.rb"
```

Each key accepts a single command string or an array of command strings, which
run in order. Omit a key or use `[]` to run no commands. Each command runs in a
separate shell, so changes from `cd` or `export` do not carry over to the next
command. Put scripts in the corresponding template directory so
they are available when the commands run. Commands also run when that template
directory is absent, allowing setup entirely through commands.

| Key              | Template                  | Working directory                  |
| ---------------- | ------------------------- | ---------------------------------- |
| `workspace`      | `workspace_template`      | Standalone problem or contest root |
| `problem`        | `problem_template`        | Each problem directory             |
| `contest`        | `contest_template`        | Contest root                       |
| `single_problem` | `single_problem_template` | Standalone problem root            |

For `cpg download`, the order is `workspace`, `problem`, then `single_problem`.
For `cpg prepare`, `workspace` and `contest` run first at the contest root, then
`problem` runs for each problem. Each command finishes before the next template
is copied, so later templates can overwrite files created by earlier commands.
After each template is copied, `.cpg.toml` is written before its setup commands
run, so scripts can read the problem or contest metadata from their working
directory. Samples and template checksums are written after the problem's setup
commands. Files created or changed by setup are included in the
unchanged-template check.

Commands run through `sh -c` in the temporary directory being prepared, which is
renamed to the final workspace path on success. Use relative paths in generated
files rather than embedding the temporary absolute path. Standard input is
closed; command stdout and stderr go to cpg's stderr so stdout contains only the
completed workspace path. A failed command skips the remaining commands and
stops the download with exit code 2.
Ctrl-C stops the command and its process group with exit code 130. Both cases
remove the temporary workspace.

You would want to set `bundle install` or `git clone https://github.com/atcoder/ac-library.git` in the workspace template,
for example.

## Login

This tool does not provide login form itself.
You need to prepare cookies for each online judge and save them in Netscape HTTP Cookie Format.
For example, you can use [cookies.txt](https://addons.mozilla.org/ja/firefox/addon/cookies-txt/) Firefox extension to export cookies.

The cookies file should be saved in `$XDG_DATA_HOME/cpg/cookies` by default.
Overridable with `$CPG_COOKIES_HOME` environment variable.
`login` verifies the session before saving `<service>.txt` with mode `600` in a
directory with mode `700`. An unsuccessful login leaves the previous cookies intact.
AtCoder Problems uses the AtCoder session; both `login atcoder` and
`login atcoder-problems` save `atcoder.txt`. Expired sessions require a fresh export.

```bash
# Login to an online judge
cpg login atcoder --cookie-file /path/to/cookies.txt
```

## Language settings

Add `[language.<name>]` tables to `$config/config.toml`. cpg selects a language
by the source file's extension. Names such as `cpp` and `ruby` are yours to
choose; each extension must match at most one language. These settings apply to
solutions, generators, reference solutions, and judge files.

| Key          | Required | Purpose                                                                     |
| ------------ | -------- | --------------------------------------------------------------------------- |
| `extensions` | Yes      | File extensions without the leading dot, such as `["cpp", "cc"]`.           |
| `run`        | Yes      | Shell command to execute the solution or script.                            |
| `compile`    | No       | Shell command to compile or check the source before running it.             |
| `preprocess` | No       | Transform the source before local execution and submission.                 |
| `presubmit`  | No       | Transform the source only for submission, after `preprocess`.               |
| `profile`    | No       | Named overrides for `compile` and `run`, selected with `--profile`.         |
| `submit`     | No       | Submission language IDs keyed by service, such as `atcoder` or `yukicoder`. |

Commands run through `sh -c` in the source file's directory. In `compile` and
`run`, `{input}` expands to the source path and `{binary}` to the same path with
its final extension removed. cpg shell-quotes both paths; leave the placeholders
unquoted in the command. When preprocessing is configured, `{input}` points to
the transformed source. Omit `compile` for interpreted languages that need no
compilation or syntax check. Compilation runs once before testing or generation.
Direct commands after `--` do not use these language settings.

### Build profiles

Use `[language.<name>.profile.<profile>]` to override `compile`, `run`, or both.
Omitted commands inherit the language's settings; a profile replaces the whole
command rather than appending flags. Select one with `cpg test --profile fast
./solution.cpp` or `cpg generate --profile fast ./generator.cpp`. The
[C++ recipe](#c-with-debugging-and-a-fast-profile) enables debugging by default
and defines a `fast` profile.

### Executable files

If no configured extension matches, executable files use the built-in
`executable` language, which runs `{input}` without compilation. This works
with extensionless binaries and scripts with execute permission and a shebang:

```bash
cpg test ./a.out
cpg generate --count 10 ./generator
```

The same fallback applies to judge files. Commands run in the executable's
directory. Non-executable files still require a matching language configuration.
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
only for submission, after `preprocess`. Each command receives the current
source on stdin and as the shell-quoted `{input}` path, runs in the source
directory, and writes the transformed UTF-8 source to stdout by default.

If the command contains `{processed}`, cpg expands it to a temporary output
file and reads the transformed source from that file instead of stdout. Leave
`{input}` and `{processed}` unquoted because cpg shell-quotes both paths. For
example, a script that accepts input and output paths can use:

```toml
[language.cpp]
extensions = ["cpp"]
preprocess = "ruby ~/expand.rb {input} {processed}"
compile = "g++ -std=c++23 -O2 -o {binary} {input}"
run = "{binary}"
```

The same placeholder is available in `presubmit`. In this mode, command stdout
goes to cpg's stderr for diagnostics. A failed command, missing or empty output
file, or non-UTF-8 output stops the operation; stdout is never used as a fallback.

Use `presubmit` instead of `preprocess` to apply a transformation only when
submitting. In a two-stage pipeline, `presubmit` receives the output of
`preprocess`. Transformations run once per source, preserve the original file,
and use temporary files with the same extension in the source directory so
relative includes keep working. The template checksum check runs before both
stages. See the [ACL recipe](#expand-atcoder-library-headers) for an example.

### Submission language IDs

Add the judge's language ID to an existing language's `submit` table. For C++23:

```toml
[language.cpp.submit]
atcoder = "6017"
yukicoder = "cpp23"
```

IDs must be quoted strings, including numeric IDs. AtCoder Problems uses the
`atcoder` entry. cpg fetches the available languages and displays their IDs if
the configured ID is missing or invalid. Copy the ID for your judge's language
and compiler version into this table, or override it for one submission with
`cpg submit ./solution.cpp --language ID`. Local compiler commands and profiles
do not select the judge's compiler. See [submitting solutions](./submissions.md).

## Recipes

Copy the settings you need into `$config/config.toml` and install the compilers
or interpreters used by their commands. If a language table already exists,
merge the settings into it instead of declaring the same table twice.

The submission IDs below were checked on 2026-09-06 against the
[AtCoder language-test submission form](https://atcoder.jp/contests/language-test-202505/submit)
and [yukicoder's language API](https://yukicoder.me/api/v1/languages).
If a contest uses a different language environment, use the IDs cpg displays
for that contest.

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
atcoder = "6082" # Python (CPython 3.13.7)
yukicoder = "python3"

# Or if you prefer PyPy:
#
# atcoder = "6083" # PyPy 3.11-v7.3.20
# yukicoder = "pypy3"
```

### Expand AtCoder Library headers

With [ac-library](https://github.com/atcoder/ac-library) checked out at
`~/ac-library`, use its [expander](https://github.com/atcoder/ac-library/blob/master/expander.py)
to inline ACL headers before local compilation and submission. This requires
Python 3 and GCC. `--console` writes the expanded source to stdout, as required
by `preprocess`.

```toml
[language.cpp]
extensions = ["cpp"]
preprocess = "python3 ~/ac-library/expander.py --console --lib ~/ac-library {input}"
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
