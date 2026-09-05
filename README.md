# Competitive Programming CLI (cpcli)

> [!WARNING]
> This project is still in early development stage, and most of the features are not implemented yet.

cpcli is a command-line interface tool for competitive programming.

This tool can:

- Download a problem from various online judges.
- Download multiple problems from a contest.
- Submit solutions to judges.
- List problems and contests you've downloaded.

Currently supported online judges:

- AtCoder
- AtCoder Problems (Virtual Contests)
- Yukicoder

## Features

### Configuration

Configuration is stored within `$XDG_CONFIG_HOME/cpcli` (called `$config` in this document) by default.
Overridable with `$CPCLI_CONFIG_HOME` environment variable.

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

### Login

This tool does not provide login form itself.
You need to prepare cookies for each online judge and save them in Netscape HTTP Cookie Format.
For example, you can use [Export Cookies](https://addons.mozilla.org/en-US/firefox/addon/export-cookies-txt/) Firefox extension to export cookies.

The cookies file should be saved in `$XDG_DATA_HOME/cpcli/cookies` by default.
Overridable with `$CPCLI_COOKIES_HOME` environment variable.
The directory will contain sensitive information, so please make sure to set the permission to `600` or `400`.

```bash
# Login to an online judge
cpcli login atcoder --cookie-file /path/to/cookies.txt
```

### Download single problem

You can download a single problem from a contest.
The downloaded problem will be saved in a directory `$root/$host/problems/$problem_id`, where:

- `$root` is the root directory of your workspace, which is specified in the configuration file.
- `$host` is the host of the online judge, such as `atcoder`.
- `$problem_id` is the problem ID, such as `abc473_f`.

The directory will contain:

- `.cpcli.toml` file, which contains metadata of the problem.
- Workspace template files.
- Problem template files.
- Single problem template files.
- Test cases (stored in `test/sample.in` and `test/sample.out`).

If multiple template files contain the same file name, the last one will overwrite the previous ones.

```bash
# Download a problem from AtCoder
cpcli download https://atcoder.jp/contests/abc473/tasks/abc473_f

# Or shortcut
cpcli d https://atcoder.jp/contests/abc473/tasks/abc473_f
```

### Download contest problems

You can download all problems from a contest.
The downloaded problems will be saved in a directory `$root/$host/contests/$contest_id/$index_$problem_id`, where:

- `$root` is the root directory of your workspace, which is specified in the configuration file.
- `$host` is the host of the online judge, such as `atcoder`.
- `$contest_id` is the contest ID, such as `abc473`.
- `$index` is the 1-indexed and zero-padded index of the problem in the contest, such as `01`.
  - This exists for sorting problems in the order they appear in the contest, for when:
    - The contest contains multiple problems from different contests (e.g. AtCoder Daily Training),
    - The problem IDs are not sorted in the order they appear in the contest (e.g. Yukicoder).
- `$problem_id` is the problem ID, such as `abc473_a`.

The contest directory will contain:

- `.cpcli.toml` file, which contains metadata of the contest.
- Workspace template files.
- Contest template files.

The problem directories will contain:

- Problem template files.
- (Note that this does not contain workspace template files)

```bash
# Download contest problems from AtCoder
cpcli prepare https://atcoder.jp/contests/abc473

# Or shortcut
cpcli p https://atcoder.jp/contests/abc473
```

### Test a solution

You can test a solution against the sample test cases.
This command will execute the command specified in the configuration file for the language of the solution file if single argument is given:

```toml
# Example configuration for C++
[language.cpp]
extensions = ["cpp"]
compile = "g++ -std=c++23 -Wall -Wextra -o {binary} {input}"
run = "./{binary} < {test_input}"

[language.cpp.profile.fast]
compile = "g++ -std=c++23 -O2 -Wall -Wextra -o {binary} {input}"
```

```bash
# Test a solution against the sample test cases
cpcli test ./solution.cpp
# -> g++ -std=c++23 -Wall -Wextra -o solution ./solution.cpp && ./solution < ./test/sample.in

# Or specify the test case directory
cpcli test --test-dir ./random ./solution.cpp
# -> g++ -std=c++23 -Wall -Wextra -o solution ./solution.cpp && ./solution < ./random/sample.in

# Or specify the profile to use for compilation
cpcli test --profile fast ./solution.cpp
# -> g++ -std=c++23 -O2 -Wall -Wextra -o solution ./solution.cpp && ./solution < ./test/sample.in
```

Or you can specify the command to execute directly after `--`:

```bash
# Test a solution against the sample test cases with custom command
cpcli test -- ruby ./solution.rb
```

### Randomly generate test cases

You can randomly generate test cases using a generator script.

```bash
# Specify generator
cpcli generate ./random.rb

# Or specify directory to save the generated test cases
cpcli generate --output ./random ./random.rb

# Or specify full command to execute
cpcli generate -- ruby ./random.rb
```

After generating test cases, you can run naive solution against the generated test cases to verify the correctness of the solution.

```bash
# Run naive solution against the generated test cases
cpcli generate --answer ./naive.rb

# Or specify full command to execute
cpcli generate --answer -- ruby ./naive.rb
```

### Submit solution

You can submit a solution to a problem.

```bash
# Submit a solution to a problem
cpcli submit ./solution.cpp
```

The problem will be detected using `.cpcli.toml` file in the directory of the solution file.
You can also specify the problem directory using `--problem` option.

```bash
# Submit a solution to a problem in another directory
cpcli submit ./solution.cpp --problem https://atcoder.jp/contests/abc473/tasks/abc473_f
```

### List results of submissions

You can list the results of your submissions.

```bash
# List results of submissions
cpcli results

# Or watch the results in real-time
cpcli results --watch
```

### List problems you've downloaded

You can list all problems you've downloaded.

```bash
# List problems you've downloaded
cpcli list

# List problems you've downloaded with full path
cpcli list --path
```

This command is for piping the output to other commands, such as `fzf`.
For example, you can create `ccd` command which changes the current working directory to the selected problem's directory.
This feature is heavily inspired by [ghq](https://github.com/x-motemen/ghq).

```bash
gcd() {
    dir="$(cpcli list --path | fzf)"
	[ -n "$dir" ] && cd "$dir"
}
```

## Installation

You can install cpcli using cargo:

```bash
# Build from source
cargo install competitive-programming-cli

# Or download the pre-built binaries using cargo-binstall
cargo binstall competitive-programming-cli
```

Or you can download the pre-built binaries from the [releases page](https://github.com/sevenc-nanashi/competitive-programming-cli), by manually or using other package managers like `mise`.

```bash
# Using mise
mise use -g github:sevenc-nanashi/competitive-programming-cli
```
