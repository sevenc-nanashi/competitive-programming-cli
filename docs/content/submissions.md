# Submissions and results

## Submit solution

You can submit a solution to a problem.

```bash
# Submit a solution to a problem
cpg submit ./solution.cpp

# Or shortcut
cpg s ./solution.cpp
```

The problem is detected using the nearest `.cpg.toml` in the solution file's
directory or its ancestors.
You can also specify the problem URL using `--problem`.

```bash
# Submit a solution to a problem in another directory
cpg submit ./solution.cpp --problem https://atcoder.jp/contests/abc473/tasks/abc473_f
```

Configure the judge's language ID for each file type, or pass `--language ID`:

```toml
[language.cpp.submit]
atcoder = "<AtCoder language ID>"
yukicoder = "<yukicoder language ID>"
```

cpg fetches the available languages and displays their IDs if the configured
ID is missing or invalid. AtCoder Problems uses the `atcoder` language setting.
A successful submission prints its ID and URL. If the judge's response is
ambiguous, cpg reports that the outcome is unknown; check `results` before
submitting again.

`submit` warns and stops if the source is identical to its saved template, including
when `--problem` is specified. To intentionally submit the unchanged file:

```bash
cpg submit ./solution.cpp --allow-submit-unchanged-solution
```

## List results of submissions

You can list the results of your submissions.

```bash
# List results of submissions
cpg results

# Or shortcut
cpg r

# Or monitor the results in an interactive terminal UI
cpg results --ui
```

The nearest `.cpg.toml` in the current directory or its ancestors selects the
problem or contest. Results include your browser submissions as well as cpg
submissions. The newest 20 are shown by default; use `--limit N` to change this.
On a terminal, both modes use aligned columns and colored status labels. URLs
appear below each submission in the normal listing. `--no-color` or `NO_COLOR`
disables colors.

The interactive monitor refreshes every three seconds. It pauses automatically
after two hours without a newly observed submission or terminal interaction,
including when there are no submissions. Regular refreshes and verdict changes
do not reset this timer. Press `p` to pause or resume polling; resuming fetches
results immediately and starts a new two-hour window. `r` refreshes once while
keeping the current pause state, and `q` quits.

When stdout is piped or redirected, both `results` and `results --ui` print a
single snapshot in the original tab-separated format, with the same header and
column order and no colors or terminal controls.
