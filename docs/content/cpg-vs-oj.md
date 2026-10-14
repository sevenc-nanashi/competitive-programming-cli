# cpg vs oj + oj-prepare

[online-judge-tools/oj](https://github.com/online-judge-tools/oj) and [online-judge-tools/template-generator](https://github.com/online-judge-tools/template-generator) (which provides `oj-prepare`)
are well-known tools for competitive programming.
I've used them for a long time, but wanted a more integrated workflow for managing workspaces, compiling solutions, and checking submissions, so I created cpg.

## What changes with cpg

- Runs as a native Rust executable, so it's very easy to install and update.
  - The [experimental oj backend](./configuration.md#experimental-oj-support) uses
    `uv` and a managed Python virtual environment.
- Places problems and contests under a fixed root directory.
  - So you don't have to care where to download problems. (and potentially lower a
    hurdle for solving problems)
- Prefixes contest problem directories with indexes.
  - Useful for virtual contests and Yukicoder contests, where problems are not ordered by ID.
- Shares workspace templates between single problems and contests.
- Compiles and tests a source file in one command, using configured language commands and build profiles.
  - In addition to running solutions, `preprocess` and `presubmit` hooks can transform source files before
    testing or submission.
- `cpg results --ui` monitors your submissions, including those made in the browser.
  - So you don't have to open submission pages.

## Command equivalents

These examples use the [oj commands](https://github.com/online-judge-tools/oj#how-to-use)
and [oj-prepare](https://github.com/online-judge-tools/template-generator#usage).
For cpg commands that take a source file, configure the language's compile/run
commands first. Direct commands after `--` do not require language settings.

| Task                                 | oj / oj-prepare                                          | cpg                                                     |
| ------------------------------------ | -------------------------------------------------------- | ------------------------------------------------------- |
| Download a problem's samples         | `oj download URL`                                        | `cpg download URL`                                      |
| Prepare a contest or a problem       | `oj-prepare URL`                                         | `cpg prepare URL`                                       |
| Compile and test C++                 | `g++ solution.cpp -o solution && oj test -c ./solution`  | `cpg test solution.cpp`<sup>1</sup>                     |
| Test a Ruby command                  | `oj test -c "ruby solution.rb"`                          | `cpg test solution.rb`                                  |
| Run test against specific input file | `oj test -c "ruby solution.rb" -i test/sample-1.in`      | `cpg test --input-path ./test/sample-1.in solution.rb`  |
| Generate inputs in `test/`           | `oj generate-input "ruby random.rb"`                     | `cpg generate random.rb`<sup>2</sup>                    |
| Generate missing answers in `test/`  | `oj generate-output -c "ruby naive.rb"`                  | `cpg generate --answer naive.rb`                        |
| Run an interactive judge             | `oj test-reactive -c "ruby solution.rb" "ruby judge.rb"` | `cpg test --interactive --judge "judge.rb" solution.rb` |
| Submit to an explicit problem        | `oj submit URL solution.cpp`                             | `cpg submit solution.cpp --problem URL`                 |

1. While cpg supports compiling and testing a source file in one command, you can still specify a command to run directly (`cpg test -- ruby solution.rb`) or compile first and then test (`g++ solution.cpp -o solution && cpg test ./solution`).
2. cpg generates random cases in the `random/` directory by default, but you can use `--test-dir` to specify a different location, including `test/`.

## Differences when switching

- Command line options are different. See [cpg commands](./command-reference.md) for details.
- There are no template system for template files. cpg copies template files as they are.
- No `oj-template` equivalent.
- No `oj downlload --system` equivalent.

## Migrating templates

`cpg init` detects `prepare.config.toml` in the XDG configuration directory for
`online-judge-tools` and asks whether to import its `[templates]` entries.
The import copies referenced local files into `problem_template`.

After importing, add [language settings](./configuration.md#language-settings) and
[import browser cookies](./configuration.md#login). Move shared project files
into `workspace_template` if they should live at the contest root.

Existing oj workspaces are not converted, but their `.in` and `.out` files can
be used directly for local testing:

```bash
cpg test --test-dir ./test -- ruby ./solution.rb
```

Submission from a directory without `.cpg.toml` requires `--problem URL`.
