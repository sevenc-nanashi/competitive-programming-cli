# Testing solutions

<Asciinema src="/competitive-programming-cli/demo/testing.cast" poster="npt:15" />

## Test a solution

You can test a solution against the sample test cases using the `test` command.

```bash
# Test a solution against the sample test cases
cpg test ./solution.cpp

# Or shorthand
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

For TLE and MLE, you can use `--time-limit` and `--memory-limit` options
to specify the time limit and memory limit for each test case.

```bash
# Test a solution against the sample test cases with time limit of 2000ms and memory limit of 256MB
cpg test --time-limit 2000 --memory-limit 256 ./solution.cpp
```

Note that these limits are not precise and may vary depending on the system
load and other factors.

You can use `--jobs N` (`-j N`) to run up to N test cases concurrently. The default is `1`.

```bash
cpg test -j 4 ./solution.cpp
```

You can use `--fast-fail` (`-f`) to stop starting new cases after the first failure.
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
The judge will receive three arguments: `{test_input}` is the `.in` path, `{solution_output}` is the solution's output path, and
`{test_output}` is the corresponding `.out` path.
If the judge command does not have these placeholders, cpg will append them to the end of the command.
Judges should return exit code 0 for accepted, otherwise return non-zero exit code for rejected.

```bash
# Test a solution against the sample test cases with custom judge
cpg test --judge ./judge.rb ./solution.rb

# Or directly specify the command to execute for each test case
cpg test --judge "ruby ./judge.rb {test_input} {solution_output} {test_output}" ./solution.rb
```

Without a custom judge, a missing `.out` skips output comparison: the case is
`AC` if the solution exits with `0`, otherwise `RE`. An existing empty `.out`
requires empty output.

Use `--input-path` (`-I`) to run a single input file instead of the test directory.
Paths are relative to the current directory; the corresponding `.out` is used
when present. Use `-I -` to read one input from standard input until EOF, without
an expected output.

```bash
cpg test -I ./example.in ./solution.cpp
printf '1 2\n' | cpg test -I - ./solution.cpp
```

Combine `-I -` with `--interactive` (`-i`) to interact with the solution manually.
Output appears immediately, and any `--judge` is ignored.

```bash
cpg test -I - -i ./solution.cpp
```

Use `--show-io` to choose when to display each case's input, expected output
(when available), and actual output:

- `always` (`a`, default if output is missing): show I/O for every case.
- `failure` (`f`, default): show I/O only for failed cases, including `WA`, `RE`, `TLE`, and `MLE`.
- `never` (`n`): hide I/O details.

Verdicts and the summary are always shown. Standard error from the solution
and judge is still streamed directly.

```bash
cpg test --show-io always ./solution.cpp
cpg test --show-io never -- ruby ./solution.rb
```

Use `--panes` (`-p`) to choose the I/O layout:

- `none` (`1`, default): display input, expected output, and actual output vertically.
- `outputs` (`2`): display input above side-by-side expected and actual output.
- `all` (`3`): display input, expected output, and actual output side by side.

Add `--query-numbers` (`-n`) to number lines relative to the expected output.

```bash
cpg test -p outputs -n ./solution.cpp
cpg test -p all -n --show-io always ./solution.cpp
```

Input and expected output align at the bottom, and expected output and actual output align at the top.
Queries are numbered relative to the expected output.

```text
  | Input:    | Expected output: | Actual output:
  | 2         :                  :
  | condition :                  :
1 | query A   | answer A         | answer A
2 | query B   | answer B         | wrong
3 :           :                  | extra
```

Use `--highlight line` or `--highlight word` (`-H`) to highlight differences on
both sides: expected output in green and actual output in red. `line` highlights
the entire differing line; `word` compares whitespace-separated words at the
same position within each corresponding line. Extra words or lines are
highlighted on the side where they exist. Input is never highlighted.

```bash
cpg test -H line ./solution.cpp
cpg test -p all -n -H word ./solution.cpp
```

For stripping trailing white-space in the output, you can use `--strip` option,
or `--strip-trailing-newline` (`-S`) to ignore only trailing CR and LF bytes
in expected and actual output. Spaces, tabs, and internal newlines are preserved.

```bash
cpg test --strip ./solution.cpp

cpg test --strip-trailing-newline ./solution.cpp
```

For CRLF/LF insensitive comparison, you can use `--ignore-line-ending` option to ignore line ending differences between the expected output and the actual output.
This is enabled by default, but you can disable it with `--no-ignore-line-ending` option.

```bash
# Test a solution against the sample test cases with CRLF/LF insensitive comparison
cpg test --ignore-line-ending ./solution.cpp
```

## Test interactive problems

<Asciinema src="/competitive-programming-cli/demo/interactive.cast" poster="npt:20" />

You can test interactive problems with custom judge.
The judge's standard input will receive the output from the solution, and the judge's standard output will be sent to the solution's standard input.
cpg will prefix `<` for the judge's output and `>` for the solution's output.
The transcript is displayed after each case according to `--show-io`.
If test files exist, the judge will receive the path as `{test_input}` and `{test_output}` arguments, and cpg will run the judge and solution for each test case.
As with custom judges, a missing `.out` is replaced with an empty temporary file for that run.
Unlike other test commands, this command can be run without test files.

```bash
# Test an interactive problem with custom judge
cpg test --interactive --judge ./judge.rb ./solution.rb
```

With `--query-numbers` (`-n`), the initial judge output is numbered `0`. The first
solution output and the judge's reply are numbered `1`, the next exchange `2`,
and so on.

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

Use `--panes outputs` (`-p 2`) to show the judge on the left and the solution on
the right. Each utterance stays on its own row in chronological order. The center
arrow is `>` for judge-to-solution and `<` for solution-to-judge:

```text
Judge:          Solution:
1 10          >
              < ? 5
HIGHER        >
```

With `--query-numbers` (`-n`), the number appears between the panes. `|` marks the
speaking side and `:` marks the empty side:

```text
Judge:              Solution:
1 10          | 0 :
              : 1 | ? 5
HIGHER        | 1 :
              : 2 | ? 8
LOWER         | 2 :
```
