# Submissions and results

## Submit solution

You can submit a solution to a problem using the `submit` command.

```bash
# Submit a solution to a problem
cpg submit ./solution.cpp

# Or shortcut
cpg s ./solution.cpp

# Submit and open the submission page in your browser
cpg submit ./solution.cpp --open
```

The problem is detected using the nearest `.cpg.toml` in the solution file's
directory or its ancestors.
You can also specify the problem URL using `--problem`.

```bash
# Submit a solution to a problem in another directory
cpg submit ./solution.cpp --problem https://atcoder.jp/contests/abc473/tasks/abc473_f
```

Note that you need to:

- be logged in to the judge to submit, and
- have configured [submission language ID](./configuration.md#submission-language-ids)
  for each file type (or pass `--language ID`).

Use `--profile NAME` (`-P NAME`) to apply a language profile's `preprocess`,
`presubmit`, and service-specific `submit` overrides.

cpg fetches the available languages and displays their IDs if the configured
ID is missing or invalid.

`submit` refuses to submit a solution that has not changed from the template.
This behavior can be overridden with `--allow-submit-unchanged-solution`.

```bash
cpg submit ./solution.cpp --allow-submit-unchanged-solution
```

## Copy a solution to the clipboard

```bash
cpg submit ./solution.cpp --clipboard
```

This runs the configured `preprocess` and `presubmit` commands, then copies the
result using the [clipboard backend](./configuration.md#clipboard).
Add `--open` to print the problem URL and open it in your browser
after copying.

This is useful for judges that require captcha verification, especially AtCoder's
past contests.

> [!TIP]
> For Codeforces, you can use [codeforces-clipboard-submission.user.js](https://sevenc7c.com/competitive-programming-cli/codeforces-clipboard-submission.user.js)
> to paste the copied solution instead of selecting the file. This is useful
> when your solution file is not easily accessible in the browser's file picker,
> such as when you're using a remote server or WSL.  
> You will need a userscript manager like [ScriptCat](https://scriptcat.org) or [Tampermonkey](https://www.tampermonkey.net) to install it.

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
problem or contest.

When stdout is piped or redirected, `results` will output the results in a
machine-readable format (TSV) instead of padded columns.
