# cpg vs oj + oj-prepare

[online-judge-tools/oj](https://github.com/online-judge-tools/oj) and [online-judge-tools/template-generator](https://github.com/online-judge-tools/template-generator) (which provides `oj-prepare`)
are well-known tools for competitive programming.
I've used them for a long time, but wanted a more integrated workflow for managing workspaces, compiling solutions, and checking submissions, so I created cpg.

## What changes with cpg

- Runs as a native Rust executable without a Python runtime. Your solution's compiler or interpreter is still needed.
- Places problems and contests under a configured root, so you can download without choosing a directory each time.
- Prefixes contest problem directories with indexes so they sort in contest order.
- Shares workspace templates between standalone problems and contests.
- Compiles and tests a source file in one command, using configured language commands and build profiles. `preprocess` and `presubmit` hooks can transform source files before testing or submission.
- Records problem and contest metadata, so `cpg open`, `cpg submit`, and `cpg results` can detect the current context. `cpg results --ui` monitors your submissions, including those made in the browser.
- Lists downloaded workspaces with `cpg list`, for use with tools such as `fzf`. Paths are relative to the root printed by `cpg config --root`.

`oj-prepare` already supports
[configurable directory patterns](https://github.com/online-judge-tools/template-generator#oj-prepare).
cpg chooses one layout under a configured root: `$root/$host/problems/$problem_id`
for standalone problems and `$root/$host/contests/$contest_id` for contests.
Contest problem directories have numeric prefixes padded to the number of digits
needed for the problem count.

`workspace_template` supplies shared files at the root of each contest or
standalone problem, while `problem_template` supplies files for every problem.
`contest_template` and `single_problem_template` add files or overrides for those
workspace types. Optional [`[setup]` commands](./configuration.md#commands-after-copying-templates)
run after each template is copied. See [Workspaces](./workspaces.md) for the layout and template order.

## Command equivalents

These examples use the [oj commands](https://github.com/online-judge-tools/oj#how-to-use)
and [oj-prepare](https://github.com/online-judge-tools/template-generator#usage).
For cpg commands that take a source file, configure the language's compile/run
commands first. Direct commands after `--` do not require language settings.

| Task                                | oj / oj-prepare                                          | cpg                                                                  |
| ----------------------------------- | -------------------------------------------------------- | -------------------------------------------------------------------- |
| Download a problem's samples        | `oj download URL`                                        | `cpg download URL`                                                   |
| Prepare a contest                   | `oj-prepare URL`                                         | `cpg prepare URL`                                                    |
| Compile and test C++                | `g++ solution.cpp -o solution && oj test -c ./solution`  | `cpg test solution.cpp`                                              |
| Test a Ruby command                 | `oj test -c "ruby solution.rb"`                          | `cpg test -- ruby solution.rb`                                       |
| Generate inputs in `test/`          | `oj generate-input "ruby random.rb"`                     | `cpg generate --dir test -- ruby random.rb`                          |
| Generate missing answers in `test/` | `oj generate-output -c "ruby naive.rb"`                  | `cpg generate --dir test --answer -- ruby naive.rb`                  |
| Run an interactive judge            | `oj test-reactive -c "ruby solution.rb" "ruby judge.rb"` | `cpg test --interactive --judge "ruby judge.rb" -- ruby solution.rb` |
| Submit to an explicit problem       | `oj submit URL solution.cpp`                             | `cpg submit solution.cpp --problem URL`                              |

`cpg download` creates a problem workspace with templates and metadata as well
as samples. Enter the printed directory before working on the solution.
Without `--dir`, cpg generates cases in `random/`; test them with
`cpg test --test-dir random solution.cpp`.

## Differences when switching

Both tools support custom judges and interactive testing, but their options are
not interchangeable. Check [oj's test options](https://github.com/online-judge-tools/oj/blob/master/onlinejudge_command/subcommand/test.py)
and `cpg test --help` when adapting scripts:

- **Limits:** `oj test -t 2` specifies seconds; use `cpg test --time-limit 2000`
  for two seconds. cpg's `--memory-limit` is in MiB and samples process-group
  memory, so its measurements need not match oj's `--mle` in MB.
- **Short flags:** oj's `-s` suppresses output details, while cpg's `-s` means
  `--strip`. Use `--show-io never` to hide cpg's I/O details, `always` to show
  every case, or `failure` (the default) to show failed cases. cpg's `-S` strips
  trailing newlines; oj's `-S` ignores spaces.

Custom judges receive input, actual output, then expected output in both tools.
You can reuse an oj judge directly:

```bash
cpg test --judge 'ruby judge.rb' -- ruby solution.rb
```

cpg copies template files as they are. It does not analyze problem statements
to generate input/output code or random generator programs, as
[oj-template does](https://github.com/online-judge-tools/template-generator#usage).
It also has no equivalent to oj's system-test download option. Keep these
differences in mind if your current workflow depends on them.

## Migrating templates

`cpg init` detects `prepare.config.toml` in the XDG configuration directory for
`online-judge-tools` and asks whether to import its `[templates]` entries. To
select a configuration explicitly:

```bash
cpg init --from-oj ~/.config/online-judge-tools/prepare.config.toml
```

The import copies referenced local files into `problem_template`, preserving
their destination names and permissions. Existing cpg configuration and
template files are kept. Directory patterns and other oj-prepare settings are
not imported. cpg does not resolve oj-template's built-in templates or render
Mako expressions; imported templates must be usable as ordinary source files.

After importing, add [language settings](./configuration.md#language-settings) and
[import browser cookies](./configuration.md#login). Move shared project files
into `workspace_template` if they should live at the contest root.

Existing oj workspaces are not converted, but their `.in` and `.out` files can
be used directly for local testing:

```bash
cpg test --test-dir ./test -- ruby ./solution.rb
```

Submission from a directory without `.cpg.toml` requires `--problem URL`.
