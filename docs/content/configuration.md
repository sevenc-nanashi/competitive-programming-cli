# Configuration and login

## Configuration

Configuration is stored within `$XDG_CONFIG_HOME/cpcli` (called `$config` in this document) by default.
Overridable with `$CPCLI_CONFIG_HOME` environment variable.
When XDG variables are unset, the configuration directory is `~/.config/cpcli`
and the data directory is `~/.local/share/cpcli`.

Run the interactive setup before downloading or listing problems:

```bash
cpcli init
```

It asks for the workspace root (default: `~/cpcli`), creates `config.toml`, the
workspace root, and the four template directories below, then prints a guide
to language settings, templates, login, and downloading problems. Relative paths
are saved as absolute paths. Re-running it keeps an existing configuration and
template files and creates any missing directories.

If configuration file of oj-prepare exists, it will be imported automatically.

You can also set the workspace root manually in `$config/config.toml`:

```toml
root = "/home/your-name/competitive-programming"
```

After setting the root, inspect the current paths with:

```bash
cpcli config
# Workspace root: /home/your-name/competitive-programming
# Configuration directory: /home/your-name/.config/cpcli
# Cookies directory: /home/your-name/.local/share/cpcli/cookies
# Workspace template directory: /home/your-name/.config/cpcli/workspace_template
# Problem template directory: /home/your-name/.config/cpcli/problem_template
# Contest template directory: /home/your-name/.config/cpcli/contest_template
# Single problem template directory: /home/your-name/.config/cpcli/single_problem_template

# Print only the workspace root, for use in scripts
cpcli config --root

# Or print only one of the other directories
cpcli config --config-dir
cpcli config --cookies-dir
cpcli config --workspace-template-dir
cpcli config --problem-template-dir
cpcli config --contest-template-dir
cpcli config --single-problem-template-dir
```

Paths are absolute and reflect the environment overrides above and in
[Login](#login). These commands do not create directories or modify settings.
The flags are mutually exclusive. The full display and `--root` require a
configured root; run `cpcli init` or set `root` first. All directory flags also work
before initialization. Template directories are located within the configuration
directory, including when `$CPCLI_CONFIG_HOME` is set.

[Language settings](./testing.md) belong in the same file. Path settings and CLI
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

## Login

This tool does not provide login form itself.
You need to prepare cookies for each online judge and save them in Netscape HTTP Cookie Format.
For example, you can use [cookies.txt](https://addons.mozilla.org/ja/firefox/addon/cookies-txt/) Firefox extension to export cookies.

The cookies file should be saved in `$XDG_DATA_HOME/cpcli/cookies` by default.
Overridable with `$CPCLI_COOKIES_HOME` environment variable.
`login` verifies the session before saving `<service>.txt` with mode `600` in a
directory with mode `700`. An unsuccessful login leaves the previous cookies intact.
AtCoder Problems uses the AtCoder session; both `login atcoder` and
`login atcoder-problems` save `atcoder.txt`. Expired sessions require a fresh export.

```bash
# Login to an online judge
cpcli login atcoder --cookie-file /path/to/cookies.txt
```
