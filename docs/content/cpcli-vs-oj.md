# cpcli vs oj + oj-prepare

[online-judge-tools/oj](https://github.com/online-judge-tools/oj) and [online-judge-tools/template-generator](https://github.com/online-judge-tools/template-generator) (which provides `oj-prepare`)
are well-known tools for competitive programming.
I've used them for a long time, but wanted a more integrated workflow for managing workspaces, compiling solutions, and checking submissions, so I created cpcli.

## What changes with cpcli

- Runs as a native Rust executable without a Python runtime. Your solution's compiler or interpreter is still needed.
- Places problems and contests under a configured root, so you can download without choosing a directory each time.
- Prefixes contest problem directories with indexes so they sort in contest order.
- Shares workspace templates between standalone problems and contests.
- Compiles and tests a source file in one command, using configured language commands and build profiles. `preprocess` and `presubmit` hooks can transform source files before testing or submission.
- Records problem and contest metadata, so `cpcli open`, `cpcli submit`, and `cpcli results` can detect the current context. `cpcli results --ui` monitors your submissions, including those made in the browser.
- Lists downloaded workspaces with `cpcli list --path`, for use with tools such as `fzf`.

`oj-prepare` already supports
[configurable directory patterns](https://github.com/online-judge-tools/template-generator#oj-prepare).
cpcli chooses one layout under a configured root: `$root/$host/problems/$problem_id`
for standalone problems and `$root/$host/contests/$contest_id` for contests.
Contest problem directories have numeric prefixes padded to the number of digits
needed for the problem count.

`workspace_template` supplies shared files at the root of each contest or
standalone problem, while `problem_template` supplies files for every problem.
`contest_template` and `single_problem_template` add files or overrides for those
workspace types. See [Workspaces](./workspaces.md) for the layout and template order.

## Command equivalents

These examples use the [oj commands](https://github.com/online-judge-tools/oj#how-to-use)
and [oj-prepare](https://github.com/online-judge-tools/template-generator#usage).
For cpcli commands that take a source file, configure the language's compile/run
commands first. Direct commands after `--` do not require language settings.

| Task                                | oj / oj-prepare                                          | cpcli                                                                  |
| ----------------------------------- | -------------------------------------------------------- | ---------------------------------------------------------------------- |
| Download a problem's samples        | `oj download URL`                                        | `cpcli download URL`                                                   |
| Prepare a contest                   | `oj-prepare URL`                                         | `cpcli prepare URL`                                                    |
| Compile and test C++                | `g++ solution.cpp -o solution && oj test -c ./solution`  | `cpcli test solution.cpp`                                              |
| Test a Ruby command                 | `oj test -c "ruby solution.rb"`                          | `cpcli test -- ruby solution.rb`                                       |
| Generate inputs in `test/`          | `oj generate-input "ruby random.rb"`                     | `cpcli generate --dir test -- ruby random.rb`                          |
| Generate missing answers in `test/` | `oj generate-output -c "ruby naive.rb"`                  | `cpcli generate --dir test --answer -- ruby naive.rb`                  |
| Run an interactive judge            | `oj test-reactive -c "ruby solution.rb" "ruby judge.rb"` | `cpcli test --interactive --judge "ruby judge.rb" -- ruby solution.rb` |
| Submit to an explicit problem       | `oj submit URL solution.cpp`                             | `cpcli submit solution.cpp --problem URL`                              |

`cpcli download` creates a problem workspace with templates and metadata as well
as samples. Enter the printed directory before working on the solution.
Without `--dir`, cpcli generates cases in `random/`; test them with
`cpcli test --test-dir random solution.cpp`.

## Differences when switching

Both tools support custom judges and interactive testing, but their options are
not interchangeable. Check [oj's test options](https://github.com/online-judge-tools/oj/blob/master/onlinejudge_command/subcommand/test.py)
and `cpcli test --help` when adapting scripts:

- **Limits:** `oj test -t 2` specifies seconds; use `cpcli test --time-limit 2000`
  for two seconds. cpcli's `--memory-limit` is in MiB and samples process-group
  memory, so its measurements need not match oj's `--mle` in MB.
- **Short flags:** oj's `-s` suppresses output details, while cpcli's `-s` means
  `--strip`. Use `--show-io never` to hide cpcli's I/O details, `always` to show
  every case, or `failure` (the default) to show failed cases. oj's `-j` selects
  parallel jobs; cpcli's `-j` selects a judge, and cases run sequentially.
- **Custom judge arguments:** oj passes input, actual output, then expected
  output. cpcli's default order is input, expected output, then actual output.
  Use explicit placeholders to reuse an oj judge without changing its script:

```bash
cpcli test --judge 'ruby judge.rb {test_input} {solution_output} {test_output}' -- ruby solution.rb
```

cpcli copies template files as they are. It does not analyze problem statements
to generate input/output code or random generator programs, as
[oj-template does](https://github.com/online-judge-tools/template-generator#usage).
It also has no equivalent to oj's system-test download option. Keep these
differences in mind if your current workflow depends on them.

## Migrating templates

`cpcli init` detects `prepare.config.toml` in the XDG configuration directory for
`online-judge-tools` and asks whether to import its `[templates]` entries. To
select a configuration explicitly:

```bash
cpcli init --from-oj ~/.config/online-judge-tools/prepare.config.toml
```

The import copies referenced local files into `problem_template`, preserving
their destination names and permissions. Existing cpcli configuration and
template files are kept. Directory patterns and other oj-prepare settings are
not imported. cpcli does not resolve oj-template's built-in templates or render
Mako expressions; imported templates must be usable as ordinary source files.

After importing, add [language settings](./testing.md) and
[import browser cookies](./configuration.md#login). Move shared project files
into `workspace_template` if they should live at the contest root.

Existing oj workspaces are not converted, but their `.in` and `.out` files can
be used directly for local testing:

```bash
cpcli test --test-dir ./test -- ruby ./solution.rb
```

Submission from a directory without `.cpcli.toml` requires `--problem URL`.
