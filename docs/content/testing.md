# Testing solutions

<Asciinema src="/competitive-programming-cli/demo/testing.cast" poster="npt:15" />

## Test a solution

Test a solution against the sample cases with `cpg test`. When you pass a source
file, cpg uses its configured language's compile and run commands. Set up
[language settings](./configuration.md#language-settings) first, or use the
[configuration recipes](./configuration.md#recipes). Executable files can also
run without a language configuration; see [executable files](./configuration.md#executable-files).

```bash
# Test a solution against the sample test cases
cpg test ./solution.cpp

# Or shortcut
cpg t ./solution.cpp

# Or specify the test case directory
cpg test --test-dir ./random ./solution.cpp

# Or specify the profile to use for compilation
cpg test --profile fast ./solution.cpp
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
sampled resident memory of the solution and its children: process-group memory
from `/proc` on Linux, process-group memory on macOS, and descendant-process
memory on Windows. Sampling pauses for 10 ms between scans. Short memory peaks
may be missed; shared pages may be counted more than once. On Windows, descendants
orphaned between samples may also be missed.
Compilation runs once before testing and is outside these limits. Limits and
Ctrl-C terminate the process group on Linux/macOS or the Job Object on Windows,
including children that belong to it.

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

Use `--panes` (`-p`) to choose the I/O layout:

- `none` (`1`, default): display input, expected output, and actual output vertically.
- `outputs` (`2`): display input above side-by-side expected and actual output.
- `all` (`3`): display input, expected output, and actual output side by side. Input
  and expected output align at the bottom; expected and actual output align at
  the top. This helps match query inputs to their answers.

Add `--line-numbers` (`-n`) to number lines relative to the expected output.
Input preamble lines have no number, and extra actual output lines continue the
numbering. This also applies to the vertical layout. With panes, one shared
number column appears on the far left.

```bash
cpg test -p outputs -n ./solution.cpp
cpg test -p all -n --show-io always ./solution.cpp
```

```text
  | Input:  | Expected output: | Actual output:
  | 2       :                  :
1 | query A | answer A         | answer A
2 | query B | answer B         | wrong
3 :         :                  | extra
```

Each separator is `:` only when the next pane on its right is padded, including
padding introduced by wrapping; otherwise it is `|`. The same rule applies
between a line number and the first pane, regardless of whether the number is
blank. A real empty line counts as data, not padding.

Panes share the terminal width equally. Long lines wrap inside their pane while
preserving the alignment of corresponding lines; continuation lines have no
number. Redirected output uses the width needed by the contents without wrapping.
An unavailable terminal width or a width too narrow for two columns per pane
produces an error. Tabs use eight-column stops, CRLF displays as a single line
ending, and terminal control sequences cannot move text outside its pane.
These display changes do not affect judging.

An empty expected output is marked `(empty)`; a missing one is marked `(missing)`
in pane layouts. In both cases input lines have no numbers, actual output starts
at `1`, and `all` places actual output after the input. `--show-io` still controls
whether any I/O details appear.

Use `--highlight line` or `--highlight word` (`-H`) to highlight differences on
both sides: expected output in green and actual output in red. `line` highlights
the entire differing line; `word` compares whitespace-separated words at the
same position within each corresponding line. Extra words or lines are
highlighted on the side where they exist. Input is never highlighted.

```bash
cpg test -H line ./solution.cpp
cpg test -p all -n -H word ./solution.cpp
```

Highlighting works with every layout and survives pane wrapping. It compares
displayed text, including `(no eol)` markers, independently of judging tolerances
and stripping options; word mode ignores differences in whitespace separators.
It is off by default, disabled by `--no-color` or `NO_COLOR`, and omitted when
output is redirected. Missing expected-output files provide no reference, so
their actual output is not highlighted. `--highlight` cannot be combined with
`--interactive`.

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
cpg will prefix `<` for the judge's output and `>` for the solution's output.
On terminals with color enabled, judge output is green and solution output is yellow.
The transcript is displayed after each case according to `--show-io`.
If test files exist, the judge will receive the path as `{test_input}` and `{test_output}` arguments, and cpg will run the judge and solution for each test case.
As with custom judges, a missing `.out` is replaced with an empty temporary file for that run.
Unlike other test commands, this command can be run without test files, and the judge will be run only once with no test files.

```bash
# Test an interactive problem with custom judge
cpg test --interactive --judge ./judge.rb ./solution.rb
```

With `--line-numbers` (`-n`), the initial judge output is numbered `0`. The first
solution output and the judge's reply are numbered `1`, the next exchange `2`,
and so on. The number increases when the speaker switches from judge to
solution. Consecutive lines from the same speaker share a number, including
output received in separate reads. Partial lines are forwarded immediately and
marked `(no eol)` in the log when the speaker changes or the interaction ends.
Input and expected-output files remain unnumbered in interactive mode.

```text
0 < initial data
1 > first query
1 < first reply
2 > second query
2 < second reply
```

```bash
cpg test --interactive -n --judge ./judge.rb ./solution.rb
```

Interactive tests support only `--panes none`; `outputs` and `all` are rejected
before running the solution or judge.
