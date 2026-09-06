# Testing solutions

## Test a solution

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
cpg test ./solution.cpp
# -> g++ -std=c++23 -Wall -Wextra -o solution ./solution.cpp && ./solution < ./test/sample-1.in

# Or shortcut
cpg t ./solution.cpp

# Or specify the test case directory
cpg test --test-dir ./random ./solution.cpp
# -> g++ -std=c++23 -Wall -Wextra -o solution ./solution.cpp && ./solution < ./random/sample-1.in

# Or specify the profile to use for compilation
cpg test --profile fast ./solution.cpp
# -> g++ -std=c++23 -O2 -Wall -Wextra -o solution ./solution.cpp && ./solution < ./test/sample-1.in
```

Or you can specify the command to execute directly after `--`:

```bash
# Test a solution against the sample test cases with custom command
cpg test -- ruby ./solution.rb
```

For TLE and MLE, you can use `--time-limit` and `--memory-limit` options to specify the time limit and memory limit for each test case.

```bash
# Test a solution against the sample test cases with time limit of 2000ms and memory limit of 256MB
cpg test --time-limit 2000 --memory-limit 256 ./solution.cpp
```

The time limit measures wall-clock time. The memory limit is in MiB and uses
the solution process group's resident memory, sampled from `/proc` every 10 ms.
Short memory peaks may be missed; shared pages may be counted more than once.
Compilation runs once before testing and is outside these limits. Limits and
Ctrl-C terminate the process group, including children that inherit that group.

Use `--jobs N` (`-j N`) to run up to N cases concurrently; the default is `1`.
Compilation and preprocessing still run once before testing. This also applies
to custom and interactive judges, with limits measured separately for each case.
Results appear as cases finish, with each verdict and its I/O displayed together.
Live output from child processes can interleave. Ctrl-C stops all running cases.

```bash
cpg test -j 4 ./solution.cpp
```

Each case reports `AC`, `WA`, `RE`, `TLE`, or `MLE`, elapsed time, and peak sampled
memory. The exit code is `0` when all cases pass, `1` when a case fails, `2` for
configuration/command errors, and `130` after interruption. Without
`--test-dir`, file-based tests read the source directory's `test` directory;
direct commands read `./test`.
Without a custom judge, a missing `.out` skips output comparison: the case is
`AC` if the solution exits with `0`, otherwise `RE`. Time and memory limits still
apply. An existing empty `.out` requires empty output.

Use `--show-io` to choose when to display each case's input, expected output
(when available), and actual output:

- `always`: show I/O for every case.
- `failure` (default): show I/O only for failed cases, including `WA`, `RE`, `TLE`, and `MLE`.
- `never`: hide I/O details.

Verdicts and the summary are always shown. Standard error from the solution
and judge is still streamed directly.
Empty I/O is displayed as a dimmed `(empty)`. Non-empty I/O without a final
newline has a dimmed `(no eol)` appended to its last line.

```bash
cpg test --show-io always ./solution.cpp
cpg test --show-io never -- ruby ./solution.rb
```

For stripping trailing white-space in the output, you can use `--strip` option to ignore trailing white-space differences between the expected output and the actual output.

```bash
# Test a solution against the sample test cases with stripping trailing white-space
cpg test --strip ./solution.cpp
```

Use `--strip-trailing-newline` (`-S`) to ignore only trailing CR and LF bytes
in expected and actual output. Spaces, tabs, and internal newlines are preserved.
This is disabled by default and does not change the displayed I/O or files passed
to a custom judge. It can be combined with `--strip` and `--ignore-line-ending`.

```bash
cpg test -S ./solution.cpp
```

For CRLF/LF insensitive comparison, you can use `--ignore-line-ending` option to ignore line ending differences between the expected output and the actual output.
This is enabled by default, but you can disable it with `--no-ignore-line-ending` option.

```bash
# Test a solution against the sample test cases with CRLF/LF insensitive comparison
cpg test --ignore-line-ending ./solution.cpp
```

Use `--fast-fail` (`-f`) to stop starting new cases after the first failure.
With parallel jobs, cases already running finish and are included in the summary.

```bash
# Test a solution against the sample test cases with fast failure
cpg test --fast-fail ./solution.cpp
```

For floating point comparison, you can use `--float-error` option to specify the acceptable error for floating point comparison.
It will allow if the absolute difference or relative difference between the expected output and the actual output is less than or equal to the specified error.

```bash
# Test a solution against the sample test cases with floating point comparison
cpg test --float-error 1e-6 ./solution.cpp

# Or allow absolute error only
cpg test --float-error 1e-6 --float-error-type absolute ./solution.cpp

# Or allow relative error only
cpg test --float-error 1e-6 --float-error-type relative ./solution.cpp
```

For custom judges, use `--judge` (`-J`) to specify the command to execute for each test case:
The judge will receive three arguments in the same order as oj: the test input file (`{test_input}`), the actual output file from the solution (`{solution_output}`), and the expected output file (`{test_output}`).
`{test_output}` is the corresponding `.out` path. If it is missing, cpg passes
an empty temporary file instead and deletes it after the case finishes.
If the judge command does not have these placeholders, cpg will append them to the end of the command.
Judges should return exit code 0 for accepted, otherwise return non-zero exit code for rejected.

```bash
# Test a solution against the sample test cases with custom judge
cpg test --judge ./judge.rb ./solution.rb

# Or directly specify the command to execute for each test case
cpg test --judge "ruby ./judge.rb {test_input} {solution_output} {test_output}" ./solution.rb
```

## Test interactive problems

You can test interactive problems with custom judge.
The judge's standard input will receive the output from the solution, and the judge's standard output will be sent to the solution's standard input.
cpg will prefix `?` for the judge's output and `!` for the solution's output.
On terminals with color enabled, judge output is green and solution output is yellow.
The transcript is displayed after each case according to `--show-io`.
If test files exist, the judge will receive the path as `{test_input}` and `{test_output}` arguments, and cpg will run the judge and solution for each test case.
As with custom judges, a missing `.out` is replaced with an empty temporary file for that run.
Unlike other test commands, this command can be run without test files, and the judge will be run only once with no test files.

```bash
# Test an interactive problem with custom judge
cpg test --interactive --judge ./judge.rb ./solution.rb
```
