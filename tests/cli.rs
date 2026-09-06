use std::{
    fs,
    io::{BufRead, BufReader, Write},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

fn command(directory: &TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cpg"));
    command
        .current_dir(directory.path())
        .env("HOME", directory.path())
        .env("CARGO_MANIFEST_DIR", directory.path())
        .env("CPG_CONFIG_HOME", "~/config")
        .env("CPG_COOKIES_HOME", "~/cookies");
    command
}

fn run(directory: &TempDir, args: &[&str], code: i32) -> Output {
    let output = command(directory).args(args).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(code),
        "{args:?}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn case(directory: &TempDir, input: &[u8], output: &[u8]) {
    fs::create_dir_all(directory.path().join("test")).unwrap();
    fs::write(directory.path().join("test/sample-1.in"), input).unwrap();
    fs::write(directory.path().join("test/sample-1.out"), output).unwrap();
}

fn run_with_input(command: &mut Command, input: &str, expected: i32) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn interactive_initialization() {
    fn initialize(directory: &TempDir, input: &str, expected: i32) -> Output {
        run_with_input(command(directory).arg("init"), input, expected)
    }

    for (input, expected_root) in [
        ("\n", "cpg"),
        ("~/my \"競プロ\" workspace\n", "my \"競プロ\" workspace"),
        ("relative workspace\n", "relative workspace"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let output = initialize(&directory, input, 0);
        assert!(String::from_utf8_lossy(&output.stderr).contains("Workspace root [~/cpg]:"));
        let root = directory.path().join(expected_root);
        let config_path = directory.path().join("config/config.toml");
        let config: toml::Value =
            toml::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(config["root"].as_str().unwrap(), root.to_str().unwrap());
        assert!(root.is_dir());
        let guide = String::from_utf8_lossy(&output.stdout);
        assert!(guide.contains(config_path.to_str().unwrap()));
        for template in [
            "workspace_template",
            "problem_template",
            "contest_template",
            "single_problem_template",
        ] {
            let path = directory.path().join("config").join(template);
            assert!(path.is_dir());
            assert!(guide.contains(path.to_str().unwrap()));
        }
        for step in [
            "[language.cpp]",
            "cpg login",
            "cpg download",
            "cpg prepare",
            "cpg test",
            "cpg submit",
        ] {
            assert!(guide.contains(step), "Missing guide step: {step}");
        }
        run(&directory, &["list"], 0);

        let original = format!(
            "{}\n# Keep my language settings\n[language.ruby]\nextensions = [\"rb\"]\nrun = \"ruby {{input}}\"\n",
            fs::read_to_string(&config_path).unwrap()
        );
        fs::write(&config_path, &original).unwrap();
        let template = directory.path().join("config/problem_template/solution.rb");
        fs::write(&template, "puts 42\n").unwrap();
        let missing = directory.path().join("config/contest_template");
        fs::remove_dir(&missing).unwrap();
        let output = run(&directory, &["init"], 0);
        assert!(!String::from_utf8_lossy(&output.stderr).contains("Workspace root ["));
        assert_eq!(fs::read_to_string(&config_path).unwrap(), original);
        assert_eq!(fs::read_to_string(template).unwrap(), "puts 42\n");
        assert!(missing.is_dir());
    }

    let directory = tempfile::tempdir().unwrap();
    let output = initialize(&directory, "", 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("no input"));
    assert!(!directory.path().join("config").exists());
    assert!(!directory.path().join("cpg").exists());
    fs::write(directory.path().join("not-a-directory"), "keep").unwrap();
    initialize(&directory, "not-a-directory\n", 2);
    assert!(!directory.path().join("config").exists());
    assert_eq!(
        fs::read_to_string(directory.path().join("not-a-directory")).unwrap(),
        "keep"
    );

    let mut child = command(&directory)
        .arg("init")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut prompt = Vec::new();
    BufReader::new(child.stderr.take().unwrap())
        .read_until(b':', &mut prompt)
        .unwrap();
    assert!(String::from_utf8_lossy(&prompt).contains("Workspace root"));
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert_eq!(status.code(), Some(130));
            break;
        }
        if started.elapsed() > Duration::from_secs(5) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("init ignored SIGINT while waiting for input");
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(!directory.path().join("config").exists());
}

#[test]
fn migrate_oj_templates() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let directory = tempfile::tempdir().unwrap();
    let oj = directory.path().join("oj-config/online-judge-tools");
    fs::create_dir_all(oj.join("template")).unwrap();
    fs::write(
        oj.join("template/main.rb"),
        "#!/usr/bin/env ruby\nputs 42\n",
    )
    .unwrap();
    fs::set_permissions(
        oj.join("template/main.rb"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    fs::write(oj.join("custom.cpp"), "int main() {}\n").unwrap();
    fs::write(directory.path().join("main.cr"), "puts 42\n").unwrap();
    let config = oj.join("prepare.config.toml");
    let original = r#"contest_directory = "./{contest_id}/{problem_id}"
problem_directory = "."
[templates]
"main.rb" = "main.rb"
"./naive.rb" = "main.rb"
"src/main.cpp" = "./custom.cpp"
"main.cr" = "~/main.cr"
"#;
    fs::write(&config, original).unwrap();
    let output = run_with_input(
        command(&directory)
            .arg("init")
            .env("XDG_CONFIG_HOME", "~/oj-config"),
        "\n\n",
        0,
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Import [templates]"));
    let destination = directory.path().join("config/problem_template");
    for file in ["main.rb", "naive.rb"] {
        assert_eq!(
            fs::read(destination.join(file)).unwrap(),
            fs::read(oj.join("template/main.rb")).unwrap()
        );
        assert_eq!(
            fs::metadata(destination.join(file))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }
    assert_eq!(
        fs::read_to_string(destination.join("src/main.cpp")).unwrap(),
        "int main() {}\n"
    );
    assert_eq!(
        fs::read_to_string(destination.join("main.cr")).unwrap(),
        "puts 42\n"
    );
    assert_eq!(fs::read_to_string(&config).unwrap(), original);

    let cpg_config = directory.path().join("config/config.toml");
    let original_cpg = fs::read(&cpg_config).unwrap();
    fs::write(destination.join("main.rb"), "# keep my edits\n").unwrap();
    let output = run(
        &directory,
        &[
            "init",
            "--from-oj",
            "~/oj-config/online-judge-tools/prepare.config.toml",
        ],
        0,
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Import [templates]"));
    assert_eq!(
        fs::read_to_string(destination.join("main.rb")).unwrap(),
        "# keep my edits\n"
    );
    assert_eq!(fs::read(cpg_config).unwrap(), original_cpg);

    run_with_input(
        command(&directory)
            .arg("init")
            .env("XDG_CONFIG_HOME", "~/oj-config")
            .env("CPG_CONFIG_HOME", "~/declined"),
        "\nn\n",
        0,
    );
    assert_eq!(
        fs::read_dir(directory.path().join("declined/problem_template"))
            .unwrap()
            .count(),
        0
    );

    for invalid in [
        "[templates]\n\"../escape\" = \"main.rb\"\n",
        "[templates]\n\"/absolute.rb\" = \"main.rb\"\n",
        "[templates]\n\"main.rb\" = 42\n",
        "[templates]\n\"main.rb\" = \"missing.rb\"\n",
        "problem_directory = \".\"\n",
    ] {
        fs::write(&config, invalid).unwrap();
        run_with_input(
            command(&directory)
                .args(["init", "--from-oj", config.to_str().unwrap()])
                .env("CPG_CONFIG_HOME", "~/invalid"),
            "\n",
            2,
        );
        assert!(!directory.path().join("invalid").exists());
        assert!(!directory.path().join("escape").exists());
    }

    // A nested destination symlink must not allow writes outside problem_template.
    fs::create_dir(directory.path().join("outside")).unwrap();
    symlink(directory.path().join("outside"), destination.join("linked")).unwrap();
    fs::write(&config, "[templates]\n\"linked/main.rb\" = \"main.rb\"\n").unwrap();
    let output = run(
        &directory,
        &["init", "--from-oj", config.to_str().unwrap()],
        2,
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("symlink"));
    assert!(!directory.path().join("outside/main.rb").exists());
    assert_eq!(
        fs::read_to_string(oj.join("template/main.rb")).unwrap(),
        "#!/usr/bin/env ruby\nputs 42\n"
    );
}

#[test]
fn configuration_schema() {
    let directory = tempfile::tempdir().unwrap();
    let output = run(&directory, &["config", "--schema"], 0);
    assert!(output.stderr.is_empty());
    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    assert!(schema["properties"]["setup"].is_object());
    assert!(schema["properties"]["language"].is_object());
    assert!(!directory.path().join("config").exists());

    run_with_input(command(&directory).arg("init"), "\n", 0);
    let config_path = directory.path().join("config/config.toml");
    assert!(
        fs::read_to_string(&config_path)
            .unwrap()
            .starts_with(&format!("#:schema {}\n", schema["$id"].as_str().unwrap()))
    );
    fs::write(config_path, "invalid configuration").unwrap();
    let unchanged = run(&directory, &["config", "--schema"], 0);
    assert_eq!(unchanged.stdout, output.stdout);
}

#[test]
fn configuration_paths() {
    let directory = tempfile::tempdir().unwrap();
    run(&directory, &["config"], 2);
    run(&directory, &["config", "--root"], 2);
    for (flag, path) in [
        ("--config-dir", "config"),
        ("--cookies-dir", "cookies"),
        ("--workspace-template-dir", "config/workspace_template"),
        ("--problem-template-dir", "config/problem_template"),
        ("--contest-template-dir", "config/contest_template"),
        (
            "--single-problem-template-dir",
            "config/single_problem_template",
        ),
    ] {
        let output = run(&directory, &["config", flag], 0);
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{}\n", directory.path().join(path).display())
        );
        assert!(output.stderr.is_empty());
        assert!(!directory.path().join(path).exists());
    }
    fs::create_dir(directory.path().join("config")).unwrap();
    let config_path = directory.path().join("config/config.toml");
    let root = directory.path().join("workspace with spaces");
    for configured_root in ["~/workspace with spaces", "workspace with spaces"] {
        let contents = format!("root = {configured_root:?}\n[setup]\nworkspace = 'exit 99'\n");
        fs::write(&config_path, &contents).unwrap();
        let output = run(&directory, &["config"], 0);
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!(
                concat!(
                    "Workspace root: {}\n",
                    "Configuration directory: {}\n",
                    "Cookies directory: {}\n",
                    "Workspace template directory: {}\n",
                    "Problem template directory: {}\n",
                    "Contest template directory: {}\n",
                    "Single problem template directory: {}\n"
                ),
                root.display(),
                directory.path().join("config").display(),
                directory.path().join("cookies").display(),
                directory.path().join("config/workspace_template").display(),
                directory.path().join("config/problem_template").display(),
                directory.path().join("config/contest_template").display(),
                directory
                    .path()
                    .join("config/single_problem_template")
                    .display()
            )
        );
        assert!(output.stderr.is_empty());
        let output = run(&directory, &["config", "--root"], 0);
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{}\n", root.display())
        );
        assert!(output.stderr.is_empty());
        assert_eq!(fs::read_to_string(&config_path).unwrap(), contents);
    }
    for args in [
        vec!["config"],
        vec!["config", "--root"],
        vec!["config", "--config-dir"],
        vec!["config", "--cookies-dir"],
        vec!["config", "--workspace-template-dir"],
        vec!["config", "--problem-template-dir"],
        vec!["config", "--contest-template-dir"],
        vec!["config", "--single-problem-template-dir"],
    ] {
        let expected = run(&directory, &args, 0);
        let output = command(&directory)
            .env("CPG_CONFIG_HOME", "config")
            .env("CPG_COOKIES_HOME", "cookies")
            .args(&args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, expected.stdout);
    }
    let flags = [
        "--schema",
        "--root",
        "--config-dir",
        "--cookies-dir",
        "--workspace-template-dir",
        "--problem-template-dir",
        "--contest-template-dir",
        "--single-problem-template-dir",
    ];
    for (index, flag) in flags.iter().enumerate() {
        for other in &flags[index + 1..] {
            run(&directory, &["config", flag, other], 2);
        }
    }
    assert!(!root.exists());
    assert!(!directory.path().join("cookies").exists());
    run(&directory, &["list", "--path"], 2);
}

#[test]
fn open_url_only() {
    let directory = tempfile::tempdir().unwrap();
    let output = run(&directory, &["open", "--url-only"], 2);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("No .cpg.toml"));

    let contest_url = "https://atcoder.jp/contests/practice";
    let problem_url = "https://atcoder.jp/contests/practice/tasks/practice_1";
    fs::create_dir_all(directory.path().join("problem/src")).unwrap();
    fs::create_dir(directory.path().join("src")).unwrap();
    fs::write(
        directory.path().join(".cpg.toml"),
        format!("kind = 'contest'\nservice = 'atcoder'\nid = 'practice'\nurl = {contest_url:?}\ntitle = 'Practice'\nproblems = []\n"),
    )
    .unwrap();
    fs::write(
        directory.path().join("problem/.cpg.toml"),
        format!("kind = 'problem'\nservice = 'atcoder'\nid = 'practice_1'\nurl = {problem_url:?}\ntitle = 'Welcome to AtCoder'\n"),
    )
    .unwrap();
    for (cwd, alias, url) in [
        ("", "open", contest_url),
        ("src", "o", contest_url),
        ("problem", "open", problem_url),
        ("problem/src", "o", problem_url),
    ] {
        let output = command(&directory)
            .current_dir(directory.path().join(cwd))
            .env("PATH", directory.path())
            .env_remove("BROWSER")
            .args([alias, "--url-only"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{url}\n")
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn shell_completion() {
    let directory = tempfile::tempdir().unwrap();
    for shell in ["bash", "elvish", "fish", "nu", "powershell", "zsh"] {
        let output = run(&directory, &["completion", shell], 0);
        let script = String::from_utf8_lossy(&output.stdout);
        assert!(script.contains(&format!("cpg __complete_word__ --shell {shell}")));
        assert!(output.stderr.is_empty());
        if shell == "bash" {
            fs::write(directory.path().join("completion.bash"), &output.stdout).unwrap();
        }
    }
    run(&directory, &["completion"], 2);
    run(&directory, &["completion", "invalid"], 2);
    for (line, expected) in [
        ("cpg co", vec!["completion", "config"]),
        ("cpg t --sho", vec!["--show-io"]),
        ("cpg test --show-io ", vec!["always", "failure", "never"]),
        ("cpg test --show-io=f", vec!["--show-io=failure"]),
        ("cpg test --no-ig", vec!["--no-ignore-line-ending"]),
        ("cpg login a", vec!["atcoder", "atcoder-problems"]),
        ("cpg config --co", vec!["--config-dir", "--cookies-dir"]),
        ("cpg test --test-dir ", vec!["\u{1}dirs"]),
        ("cpg generate --dir ", vec!["\u{1}dirs"]),
        ("cpg login atcoder --cookie-file ", vec!["\u{1}files"]),
        ("cpg submit sol", vec!["\u{1}files"]),
        ("cpg download https:", vec![]),
    ] {
        let output = command(&directory)
            .env("CPG_CONFIG_HOME", "")
            .env("CPG_COOKIES_HOME", "")
            .args(["__complete_word__", "--shell", "bash", "--line", line])
            .output()
            .unwrap();
        assert!(output.status.success(), "{line}: {output:?}");
        assert!(output.stderr.is_empty(), "{line}: {output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        if expected.is_empty() {
            assert!(stdout.is_empty(), "{line}: {stdout}");
        }
        for candidate in expected {
            assert!(
                stdout.lines().any(|line| line == candidate),
                "{line}: {stdout}"
            );
        }
    }
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_cpg"), directory.path().join("cpg")).unwrap();
    fs::write(directory.path().join("solution file.cpp"), "").unwrap();
    fs::write(directory.path().join("case.txt"), "").unwrap();
    fs::create_dir(directory.path().join("cases")).unwrap();
    let output = Command::new("/bin/bash")
        .current_dir(directory.path())
        .env("PATH", directory.path())
        .args([
            "--noprofile",
            "--norc",
            "-c",
            r#"
source ./completion.bash
COMP_LINE='cpg test sol'
COMP_POINT=${#COMP_LINE}
COMP_WORDS=(cpg test sol)
COMP_CWORD=2
_usage_complete_cpg
[[ ${COMPREPLY[*]} == 'solution file.cpp' ]] || exit 1
COMP_LINE='cpg test --test-dir ca'
COMP_POINT=${#COMP_LINE}
COMP_WORDS=(cpg test --test-dir ca)
COMP_CWORD=3
_usage_complete_cpg
[[ ${COMPREPLY[*]} == cases ]] || exit 1
COMP_LINE='cpg test --show-io f'
COMP_POINT=${#COMP_LINE}
COMP_WORDS=(cpg test --show-io f)
_usage_complete_cpg
[[ ${COMPREPLY[*]} == failure ]]
"#,
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(!directory.path().join("config").exists());
    assert!(!directory.path().join("cookies").exists());
}

#[test]
fn missing_cookies_warning() {
    for (url, service, host) in [
        ("https://atcoder.jp/invalid", "atcoder", "atcoder.jp"),
        ("https://kenkoooo.com/invalid", "atcoder", "atcoder.jp"),
        ("https://yukicoder.me/invalid", "yukicoder", "yukicoder.me"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let output = run(&directory, &["download", url], 2);
        let logs = String::from_utf8_lossy(&output.stderr);
        let path = directory.path().join(format!("cookies/{service}.txt"));
        assert!(
            logs.contains(&format!(
                "Missing cookies for {service}: {}",
                path.display()
            )),
            "{logs}"
        );
        assert!(logs.contains(&format!("cpg login {service} --cookie-file <path>")));
        assert_eq!(logs.matches("Missing cookies").count(), 1, "{logs}");
        assert!(output.stdout.is_empty());
        assert!(!path.exists());

        fs::create_dir(directory.path().join("cookies")).unwrap();
        fs::write(
            &path,
            format!("{host}\tFALSE\t/\tTRUE\t0\tsession\ttest-session\n"),
        )
        .unwrap();
        let output = run(&directory, &["download", url], 2);
        let logs = String::from_utf8_lossy(&output.stderr);
        assert!(!logs.contains("Missing cookies"), "{logs}");
    }
}

#[test]
fn cli_contract_and_local_judging() {
    let directory = tempfile::tempdir().unwrap();
    run(&directory, &["--version"], 0);
    for name in [
        "init",
        "login",
        "download",
        "d",
        "prepare",
        "p",
        "test",
        "t",
        "generate",
        "g",
        "submit",
        "s",
        "results",
        "r",
        "list",
        "open",
        "o",
        "config",
        "completion",
    ] {
        run(&directory, &[name, "--help"], 0);
    }
    let help = run(&directory, &["results", "--help"], 0);
    assert!(String::from_utf8_lossy(&help.stdout).contains("--ui"));
    run(&directory, &["results", "--watch"], 2);
    for options in [
        vec!["--time-limit", "0"],
        vec!["--memory-limit", "0"],
        vec!["--jobs", "0"],
        vec!["-j", "invalid"],
        vec!["--float-error", "NaN"],
        vec!["--float-error", "inf"],
        vec!["--float-error", "-1"],
        vec!["--interactive"],
    ] {
        let mut args = vec!["test"];
        args.extend(options);
        args.extend(["--", "cat"]);
        run(&directory, &args, 2);
    }
    run(&directory, &["test", "file.rb", "--", "cat"], 2);
    run(&directory, &["test", "--", "cat"], 2);
    case(&directory, b"hello\r\n", b"hello\n");
    let output = run(&directory, &["t", "--", "cat"], 0);
    assert!(String::from_utf8_lossy(&output.stdout).contains("sample-1: AC ("));
    assert!(!output.stdout.contains(&0x1b));
    run(
        &directory,
        &["test", "--no-ignore-line-ending", "--", "cat"],
        1,
    );
    run(&directory, &["test", "--", "sh", "-c", "exit 7"], 1);
    let output = run(
        &directory,
        &["test", "--no-color", "--", "/definitely/missing"],
        2,
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .lines()
            .any(|line| line.starts_with("x) [cpg] "))
    );
    assert!(!output.stderr.contains(&0x1b));
    case(&directory, b"hello  \n\n", b"hello\n");
    run(&directory, &["test", "--strip", "--", "cat"], 0);
    case(&directory, b"1.0000001\n", b"1\n");
    run(
        &directory,
        &["test", "--float-error", "1e-6", "--", "cat"],
        0,
    );
    run(
        &directory,
        &["test", "--float-error", "1e-9", "--", "cat"],
        1,
    );
    case(&directory, b"1\n", b"0\n");
    run(
        &directory,
        &[
            "test",
            "--float-error",
            "2",
            "--float-error-type",
            "absolute",
            "--",
            "cat",
        ],
        0,
    );
    run(
        &directory,
        &[
            "test",
            "--float-error",
            "2",
            "--float-error-type",
            "relative",
            "--",
            "cat",
        ],
        1,
    );
    case(&directory, b"", b"different\n");
    let output = run(
        &directory,
        &[
            "test",
            "--judge",
            "test -f {test_input} && test -f {test_output} && test -f {solution_output}",
            "--",
            "true",
        ],
        0,
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("All 1 test case(s) passed"));
    fs::write(directory.path().join("test/second.in"), "").unwrap();
    fs::write(directory.path().join("test/second.out"), "wrong").unwrap();
    let output = run(&directory, &["test", "--fast-fail", "--", "true"], 1);
    assert!(String::from_utf8_lossy(&output.stderr).contains("0 of 1 test case(s) passed"));
}

#[test]
fn strip_trailing_newline() {
    let directory = tempfile::tempdir().unwrap();
    case(&directory, b"answer\n", b"answer");
    run(&directory, &["test", "--", "cat"], 1);
    for flag in ["--strip-trailing-newline", "-S"] {
        for (actual, expected, code) in [
            ("answer\n", "answer", 0),
            ("answer\r\n\r\n", "answer", 0),
            ("\r\n", "", 0),
            ("answer \n", "answer", 1),
            ("answer\t\n", "answer", 1),
            ("a\nb\n", "ab", 1),
        ] {
            case(&directory, actual.as_bytes(), expected.as_bytes());
            run(&directory, &["test", flag, "--", "cat"], code);
        }
    }
    case(&directory, b"a\r\nb\r\n", b"a\nb");
    run(&directory, &["test", "-S", "--", "cat"], 0);
    run(
        &directory,
        &["test", "-S", "--no-ignore-line-ending", "--", "cat"],
        1,
    );
    case(&directory, b"answer \n\n", b"answer");
    run(&directory, &["test", "-s", "-S", "--", "cat"], 0);
}

#[test]
fn parallel_jobs() {
    let directory = tempfile::tempdir().unwrap();
    let state = directory.path().join("state");
    fs::write(
        directory.path().join("worker.rb"),
        r##"
def update(delta)
  File.open('state', 'r+') do |file|
    file.flock(File::LOCK_EX)
    active, peak, started = file.read.split.map(&:to_i)
    active += delta
    peak = [peak, active].max
    started += 1 if delta > 0
    file.rewind
    file.truncate(0)
    file.write("#{active} #{peak} #{started}")
    started
  end
end
data = STDIN.read
update(1)
deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 5
until update(0) >= Integer(ARGV.fetch(0))
  abort 'jobs did not overlap' if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
  sleep 0.01
end
print(data.empty? ? "generated\n" : data)
update(-1)
"##,
    )
    .unwrap();
    fs::create_dir(directory.path().join("test")).unwrap();
    for n in 1..=4 {
        for extension in ["in", "out"] {
            fs::write(
                directory.path().join(format!("test/{n}.{extension}")),
                format!("case-{n}\n"),
            )
            .unwrap();
        }
    }
    for (flags, concurrency) in [
        (vec![], "1"),
        (vec!["--jobs", "2"], "2"),
        (vec!["-j", "2"], "2"),
    ] {
        fs::write(&state, "0 0 0").unwrap();
        let mut args = vec!["test", "--show-io", "always"];
        args.extend(flags);
        args.extend(["--", "ruby", "worker.rb", concurrency]);
        let output = run(&directory, &args, 0);
        assert_eq!(
            fs::read_to_string(&state).unwrap(),
            format!("0 {concurrency} 4")
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        for n in 1..=4 {
            assert!(
                stdout.contains(&format!(
                    "Input:\ncase-{n}\n\nExpected output:\ncase-{n}\n\nActual output:\ncase-{n}\n\n"
                )),
                "{stdout}"
            );
        }
    }
    run(
        &directory,
        &[
            "test",
            "-j",
            "2",
            "-J",
            "cmp {solution_output} {test_output}",
            "--",
            "cat",
        ],
        0,
    );
    run(
        &directory,
        &[
            "test",
            "-j",
            "2",
            "--interactive",
            "-J",
            "test -f {test_input}; printf 'question\\n'; read answer; test \"$answer\" = answer",
            "--",
            "sh",
            "-c",
            "read question; printf 'answer\\n'",
        ],
        0,
    );
    for n in 1..=4 {
        fs::write(directory.path().join(format!("test/{n}.out")), "wrong").unwrap();
    }
    fs::write(&state, "0 0 0").unwrap();
    let output = run(
        &directory,
        &[
            "test",
            "-j",
            "2",
            "--fast-fail",
            "--",
            "ruby",
            "worker.rb",
            "2",
        ],
        1,
    );
    assert_eq!(fs::read_to_string(&state).unwrap(), "0 2 2");
    assert!(String::from_utf8_lossy(&output.stderr).contains("0 of 2 test case(s) passed"));

    run(&directory, &["generate", "-j", "0", "--", "true"], 2);
    let generated = directory.path().join("random");
    fs::create_dir(&generated).unwrap();
    fs::write(generated.join("random-0001.in"), "keep").unwrap();
    fs::write(generated.join("random-0002.out"), "keep orphan answer").unwrap();
    fs::write(&state, "0 0 0").unwrap();
    run(
        &directory,
        &[
            "generate",
            "-j",
            "2",
            "--count",
            "4",
            "--",
            "ruby",
            "worker.rb",
            "2",
        ],
        0,
    );
    assert_eq!(fs::read_to_string(&state).unwrap(), "0 2 4");
    for n in 3..=6 {
        assert_eq!(
            fs::read_to_string(generated.join(format!("random-{n:04}.in"))).unwrap(),
            "generated\n"
        );
    }
    fs::write(&state, "0 0 0").unwrap();
    run(
        &directory,
        &[
            "generate",
            "--jobs",
            "2",
            "--answer",
            "--",
            "ruby",
            "worker.rb",
            "2",
        ],
        0,
    );
    assert_eq!(fs::read_to_string(&state).unwrap(), "0 2 5");
    assert_eq!(
        fs::read_to_string(generated.join("random-0001.in")).unwrap(),
        "keep"
    );
    assert_eq!(
        fs::read_to_string(generated.join("random-0001.out")).unwrap(),
        "keep"
    );
    assert_eq!(
        fs::read_to_string(generated.join("random-0002.out")).unwrap(),
        "keep orphan answer"
    );
    for n in 3..=6 {
        assert_eq!(
            fs::read_to_string(generated.join(format!("random-{n:04}.out"))).unwrap(),
            "generated\n"
        );
    }
    run(
        &directory,
        &[
            "generate",
            "--jobs",
            "2",
            "--count",
            "4",
            "--dir",
            "failed",
            "--",
            "sh",
            "-c",
            "printf partial; exit 1",
        ],
        2,
    );
    assert_eq!(
        fs::read_dir(directory.path().join("failed"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn show_io() {
    let directory = tempfile::tempdir().unwrap();
    case(&directory, b"input\n", b"answer\n");
    run(
        &directory,
        &["test", "--show-io", "invalid", "--", "cat"],
        2,
    );
    run(&directory, &["test", "--show-io"], 2);

    for mode in [None, Some("always"), Some("failure"), Some("never")] {
        for (solution, judge, actual, verdict) in [
            ("printf 'answer\\n'", None, "answer\n", "AC"),
            ("printf wrong", None, "wrong (no eol)", "WA"),
            ("printf partial; exit 7", None, "partial (no eol)", "RE"),
            ("printf 'answer\\n'", Some("false"), "answer\n", "WA"),
        ] {
            let mut args = vec!["test"];
            if let Some(mode) = mode {
                args.extend(["--show-io", mode]);
            }
            if let Some(judge) = judge {
                args.extend(["--judge", judge]);
            }
            args.extend(["--", "sh", "-c", solution]);
            let output = run(&directory, &args, i32::from(verdict != "AC"));
            let stdout = String::from_utf8_lossy(&output.stdout);
            let shown = mode != Some("never") && (mode == Some("always") || verdict != "AC");
            assert!(stdout.contains(&format!("sample-1: {verdict} (")));
            for details in [
                "Input:\ninput\n".to_owned(),
                "Expected output:\nanswer\n".to_owned(),
                format!("Actual output:\n{actual}\n"),
            ] {
                assert_eq!(stdout.contains(&details), shown, "{args:?}\n{stdout}");
            }
            let summary = if verdict == "AC" {
                "All 1 test case(s) passed"
            } else {
                "0 of 1 test case(s) passed"
            };
            assert!(String::from_utf8_lossy(&output.stderr).contains(summary));
        }
    }

    fs::remove_file(directory.path().join("test/sample-1.out")).unwrap();
    let output = run(&directory, &["test", "--show-io", "always", "--", "cat"], 0);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Input:\ninput\n"));
    assert!(stdout.contains("Actual output:\ninput\n"));
    assert!(!stdout.contains("Expected output:"));

    fs::remove_dir_all(directory.path().join("test")).unwrap();
    for mode in [None, Some("always"), Some("failure"), Some("never")] {
        for code in [0, 1] {
            let judge = format!("printf 'question\\n'; read answer; exit {code}");
            let mut args = vec![
                "test",
                "--interactive",
                "--judge",
                &judge,
                "--time-limit",
                "2000",
            ];
            if let Some(mode) = mode {
                args.extend(["--show-io", mode]);
            }
            args.extend(["--", "sh", "-c", "read question; printf 'answer\\n'"]);
            let output = run(&directory, &args, code);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let shown = mode != Some("never") && (mode == Some("always") || code != 0);
            for text in ["Interaction:", "< question", "> answer"] {
                assert_eq!(stdout.contains(text), shown, "{args:?}\n{stdout}");
                assert!(!stderr.contains(text), "{args:?}\n{stderr}");
            }
            assert_eq!(
                stdout.contains("Interaction:\n< question\n> answer\n\n"),
                shown,
                "{args:?}\n{stdout}"
            );
            let summary = if code == 0 {
                "All 1 test case(s) passed"
            } else {
                "0 of 1 test case(s) passed"
            };
            assert!(stderr.contains(summary));
        }
    }

    for (contents, displayed) in [
        ("", "(empty)"),
        (" \n", " \n"),
        ("answer", "answer (no eol)"),
        ("answer\n", "answer\n"),
        ("answer\r\n", "answer\r\n"),
        ("first\nlast", "first\nlast (no eol)"),
    ] {
        case(&directory, contents.as_bytes(), contents.as_bytes());
        let output = run(&directory, &["test", "--show-io", "always", "--", "cat"], 0);
        let stdout = String::from_utf8_lossy(&output.stdout);
        for label in ["Input", "Expected output", "Actual output"] {
            assert!(
                stdout.contains(&format!("{label}:\n{displayed}\n")),
                "{stdout}"
            );
        }
        assert!(!stdout.contains('\u{1b}'));
    }
}

#[test]
fn judge_argument_order() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("config")).unwrap();
    fs::write(
        directory.path().join("config/config.toml"),
        "[language.ruby]\nextensions = ['rb']\nrun = 'ruby {input}'\n",
    )
    .unwrap();
    for expected in ["expected\n", ""] {
        case(&directory, b"input\n", expected.as_bytes());
        if expected.is_empty() {
            fs::remove_file(directory.path().join("test/sample-1.out")).unwrap();
        }
        for name in ["judge", "judge.rb"] {
            let path = directory.path().join(name);
            fs::write(&path, format!("#!/usr/bin/env ruby\nabort ARGV.inspect unless ARGV.map {{ |path| File.read(path) }} == [\"input\\n\", \"actual\\n\", {expected:?}]\n")).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        for judge in [
            "./judge",
            "./judge.rb",
            "ruby ./judge.rb",
            "ruby ./judge.rb {test_input} {solution_output} {test_output}",
        ] {
            run(
                &directory,
                &["test", "--judge", judge, "--", "printf", "actual\n"],
                0,
            );
        }
    }
}

#[test]
fn missing_expected_outputs() {
    let directory = tempfile::tempdir().unwrap();
    case(&directory, b"hello\n", b"hello\n");
    let expected = directory.path().join("test/sample-1.out");
    fs::remove_file(&expected).unwrap();
    let output = run(&directory, &["test", "--", "cat"], 0);
    assert!(String::from_utf8_lossy(&output.stdout).contains("sample-1: AC ("));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Missing expected output for sample-1")
    );
    let output = run(&directory, &["test", "--", "false"], 1);
    assert!(String::from_utf8_lossy(&output.stdout).contains("sample-1: RE ("));
    assert!(!expected.exists());
    run(
        &directory,
        &[
            "test",
            "--judge",
            "printf '%s' {test_output} > expected-path; test -f {test_output} && test ! -s {test_output} && cmp {test_input} {solution_output}",
            "--",
            "cat",
        ],
        0,
    );
    let temporary = fs::read_to_string(directory.path().join("expected-path")).unwrap();
    assert!(!std::path::Path::new(&temporary).exists());
    assert!(!expected.exists());
    let output = run(
        &directory,
        &[
            "test",
            "--judge",
            "printf '%s' {test_output} > expected-path; false",
            "--",
            "cat",
        ],
        1,
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("sample-1: WA ("));
    let temporary = fs::read_to_string(directory.path().join("expected-path")).unwrap();
    assert!(!std::path::Path::new(&temporary).exists());
    assert!(!expected.exists());
    fs::write(
        directory.path().join("judge.rb"),
        "abort unless ARGV.size == 3 && File.read(ARGV[2]).empty? && File.read(ARGV[0]) == File.read(ARGV[1])",
    )
    .unwrap();
    run(
        &directory,
        &["test", "--judge", "ruby ./judge.rb", "--", "cat"],
        0,
    );
    assert!(!expected.exists());
    run(&directory, &["test", "--", "cat"], 0);
    run(
        &directory,
        &["generate", "--dir", "test", "--answer", "--", "cat"],
        0,
    );
    run(&directory, &["test", "--judge", "true", "--", "cat"], 0);
    assert_eq!(fs::read(&expected).unwrap(), b"hello\n");
    fs::remove_file(&expected).unwrap();

    fs::write(directory.path().join("test/second.in"), "hello\n").unwrap();
    fs::write(directory.path().join("test/second.out"), "").unwrap();
    let output = run(&directory, &["test", "--", "cat"], 1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sample-1: AC (") && stdout.contains("second: WA ("));
    assert!(String::from_utf8_lossy(&output.stderr).contains("1 of 2 test case(s) passed"));

    fs::create_dir(&expected).unwrap();
    run(&directory, &["test", "--", "cat"], 2);
}

#[test]
fn executable_fallback() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let source = "#!/bin/sh\nprintf 'sample\\n'\n";
    case(&directory, b"sample\n", b"sample\n");
    for name in ["solution", "space ' name.bin"] {
        let path = directory.path().join(name);
        fs::write(&path, source).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        run(&directory, &["test", name], 0);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        let output = run(&directory, &["test", name], 2);
        assert!(String::from_utf8_lossy(&output.stderr).contains("No language configured"));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    run(&directory, &["generate", "--count", "1", "solution"], 0);
    assert_eq!(
        fs::read_to_string(directory.path().join("random/random-0001.in")).unwrap(),
        "sample\n"
    );
    fs::create_dir(directory.path().join("config")).unwrap();
    let config_path = directory.path().join("config/config.toml");
    let config = r#"
[language.executable]
extensions = []
run = "printf 'override\\n'"
[language.executable.profile.debug]
run = "printf 'profile\\n'"
[language.executable.profile.compile]
compile = "printf overwritten > {binary}"
[language.configured]
extensions = ["bin"]
run = "printf 'configured\\n'"
"#;
    fs::write(&config_path, config).unwrap();
    for (args, expected) in [
        (vec!["test", "solution"], "override\n"),
        (vec!["test", "--profile", "debug", "solution"], "profile\n"),
        (vec!["test", "space ' name.bin"], "configured\n"),
    ] {
        case(&directory, b"sample\n", expected.as_bytes());
        run(&directory, &args, 0);
    }
    let output = run(&directory, &["test", "--profile", "compile", "solution"], 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("overwrite the source file"));
    assert_eq!(
        fs::read_to_string(directory.path().join("solution")).unwrap(),
        source
    );
    fs::write(
        config_path,
        format!("{config}\n[language.duplicate]\nextensions = ['bin']\nrun = 'false'\n"),
    )
    .unwrap();
    let output = run(&directory, &["test", "space ' name.bin"], 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("Multiple languages"));
}

#[test]
fn configured_programs_and_generation() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("config")).unwrap();
    fs::write(
        directory.path().join("config/config.toml"),
        r#"
[language.ruby]
extensions = ["rb"]
compile = "ruby -c {input}"
run = "ruby {input}"
[language.ruby.profile.broken]
compile = "false"
[language.cpp]
extensions = ["cpp"]
compile = "g++ -std=c++23 -o {binary} {input}"
run = "{binary}"
"#,
    )
    .unwrap();
    fs::write(directory.path().join("space ' name.rb"), "print STDIN.read").unwrap();
    case(&directory, b"sample\n", b"sample\n");
    run(
        &directory,
        &["test", "--test-dir", "~/test", "~/space ' name.rb"],
        0,
    );
    fs::write(
        directory.path().join("judge.rb"),
        "abort ARGV.inspect unless ARGV.size == 3; ARGV.each { |path| data = File.binread(path); abort \"#{path}: #{data.inspect}\" unless data == \"sample\\n\" }",
    )
    .unwrap();
    run(
        &directory,
        &["test", "--judge", "~/judge.rb", "~/space ' name.rb"],
        0,
    );
    run(
        &directory,
        &["test", "--profile", "broken", "space ' name.rb"],
        2,
    );
    fs::write(
        directory.path().join("solution.cpp"),
        "#include <iostream>\nint main() { std::cout << std::cin.rdbuf(); }\n",
    )
    .unwrap();
    run(&directory, &["test", "solution.cpp"], 0);
    run(&directory, &["test", "solution"], 0);
    run(
        &directory,
        &[
            "g", "--dir", "~/random", "--count", "2", "--", "ruby", "-e", "puts 42",
        ],
        0,
    );
    run(
        &directory,
        &[
            "generate", "--dir", "random", "--count", "1", "--", "ruby", "-e", "puts 43",
        ],
        0,
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("random/random-0003.in")).unwrap(),
        "43\n"
    );
    run(
        &directory,
        &["generate", "--dir", "random", "--answer", "--", "cat"],
        0,
    );
    run(
        &directory,
        &["test", "--test-dir", "random", "--", "cat"],
        0,
    );
    run(
        &directory,
        &["generate", "--dir", "random", "--answer", "--", "false"],
        0,
    );
    let before = fs::read_dir(directory.path().join("random"))
        .unwrap()
        .count();
    run(
        &directory,
        &[
            "generate",
            "--dir",
            "random",
            "--",
            "sh",
            "-c",
            "echo partial; exit 1",
        ],
        2,
    );
    assert_eq!(
        fs::read_dir(directory.path().join("random"))
            .unwrap()
            .count(),
        before
    );
    run(&directory, &["generate", "--count", "0", "--", "true"], 2);
    fs::write(directory.path().join("root.in"), "root\n").unwrap();
    fs::write(directory.path().join("root.out"), "root\n").unwrap();
    run(&directory, &["test", "--test-dir", "~", "--", "cat"], 0);
    let output = command(&directory)
        .env_remove("HOME")
        .env("CPG_CONFIG_HOME", directory.path().join("config"))
        .env("CPG_COOKIES_HOME", directory.path().join("cookies"))
        .args(["test", "~/space ' name.rb"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("HOME must be set to expand ~"));

    let config_path = directory.path().join("config/config.toml");
    let mut config: toml::Value =
        toml::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    config["language"]["cpp"]
        .as_table_mut()
        .unwrap()
        .insert("preprocess".into(), "ruby ~/expand.rb {input}".into());
    config["language"]["cpp"]
        .as_table_mut()
        .unwrap()
        .insert("presubmit".into(), "false".into());
    fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();
    fs::write(directory.path().join("expand.rb"), "abort unless File.extname(ARGV.fetch(0)) == '.cpp'; File.open('calls', 'a') { |f| f.puts 'preprocess' }; warn 'diagnostic'; print File.read(ARGV[0]).gsub('TOKEN', File.read('replacement.txt').strip)").unwrap();
    fs::write(directory.path().join("replacement.txt"), "MESSAGE").unwrap();
    fs::write(directory.path().join("local.hpp"), "#define MESSAGE 7\n").unwrap();
    let cpp = "#include \"local.hpp\"\n#include <iostream>\nint main() { std::cout << TOKEN << '\\n'; }\n";
    fs::write(directory.path().join("solution.cpp"), cpp).unwrap();
    case(&directory, b"7\n", b"7\n");
    fs::write(directory.path().join("test/second.in"), "7\n").unwrap();
    fs::write(directory.path().join("test/second.out"), "7\n").unwrap();
    run(&directory, &["test", "-j", "2", "solution.cpp"], 0);
    assert_eq!(
        fs::read_to_string(directory.path().join("calls")).unwrap(),
        "preprocess\n"
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("solution.cpp")).unwrap(),
        cpp
    );

    config["language"]["ruby"]
        .as_table_mut()
        .unwrap()
        .insert("preprocess".into(), "ruby ~/rewrite.rb {input}".into());
    config["language"]["ruby"]
        .as_table_mut()
        .unwrap()
        .insert("presubmit".into(), "false".into());
    fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();
    fs::write(directory.path().join("rewrite.rb"), "code = File.read(ARGV.fetch(0)); puts({ 'COPY' => 'print STDIN.read', 'JUDGE' => 'abort unless ARGV.size == 3 && File.read(ARGV[1]) == File.read(ARGV[2])' }.fetch(code))").unwrap();
    fs::write(directory.path().join("space ' name.rb"), "COPY").unwrap();
    fs::write(directory.path().join("judge.rb"), "JUDGE").unwrap();
    run(
        &directory,
        &["test", "--judge", "~/judge.rb", "~/space ' name.rb"],
        0,
    );
    for preprocess in ["printf partial; exit 1", "true"] {
        config["language"]["ruby"]["preprocess"] = preprocess.into();
        fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();
        run(&directory, &["test", "space ' name.rb"], 2);
    }
    assert!(fs::read_dir(directory.path()).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("cpg_preprocessed_")
    }));
}

#[test]
fn limits_interactive_and_cleanup() {
    let directory = tempfile::tempdir().unwrap();
    case(&directory, b"", b"");
    fs::remove_file(directory.path().join("test/sample-1.out")).unwrap();
    let output = run(
        &directory,
        &[
            "test",
            "--time-limit",
            "100",
            "--",
            "sh",
            "-c",
            "sleep 10 & echo $! > child.pid; wait",
        ],
        1,
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("TLE"));
    let pid: i32 = fs::read_to_string(directory.path().join("child.pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let stopped = match procfs::process::Process::new(pid).and_then(|p| p.stat()) {
        Ok(stat) => stat.state == 'Z',
        Err(procfs::ProcError::NotFound(_)) => true,
        Err(error) => panic!("{error}"),
    };
    assert!(stopped, "child {pid} is still running");
    let output = run(
        &directory,
        &[
            "test",
            "--memory-limit",
            "32",
            "--time-limit",
            "5000",
            "--",
            "ruby",
            "-e",
            "x = 'x' * 100_000_000; sleep 10",
        ],
        1,
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("MLE"));
    fs::write(
        directory.path().join("judge.rb"),
        "$stdout.sync = true\nprint '7'\nexit(STDIN.read(1) == '8' ? 0 : 1)\n",
    )
    .unwrap();
    fs::remove_dir_all(directory.path().join("test")).unwrap();
    let output = run(
        &directory,
        &[
            "test",
            "--interactive",
            "--show-io",
            "always",
            "--judge",
            "ruby ./judge.rb",
            "--time-limit",
            "2000",
            "--",
            "ruby",
            "-e",
            "$stdout.sync=true; print(STDIN.read(1).to_i + 1)",
        ],
        0,
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Interaction:\n< 7> 8 (no eol)\n"),
        "{stdout}"
    );
    case(&directory, b"", b"");
    fs::remove_file(directory.path().join("test/sample-1.out")).unwrap();
    run(
        &directory,
        &[
            "test",
            "--interactive",
            "--judge",
            "printf '%s' {test_output} > expected-path; test -f {test_input} && test -f {test_output} && test ! -s {test_output} && ruby ./judge.rb",
            "--time-limit",
            "2000",
            "--",
            "ruby",
            "-e",
            "$stdout.sync=true; print(STDIN.read(1).to_i + 1)",
        ],
        0,
    );
    run(
        &directory,
        &[
            "test",
            "--interactive",
            "--judge",
            "printf '%s' {test_output} > expected-path; exit 1",
            "--time-limit",
            "100",
            "--",
            "sleep",
            "10",
        ],
        1,
    );
    let temporary = fs::read_to_string(directory.path().join("expected-path")).unwrap();
    assert!(!std::path::Path::new(&temporary).exists());
    assert!(!directory.path().join("test/sample-1.out").exists());
    fs::write(directory.path().join("test/second.in"), "").unwrap();
    for mut args in [
        vec!["test", "-j", "2"],
        vec![
            "generate",
            "-j",
            "2",
            "--count",
            "4",
            "--dir",
            "interrupted",
        ],
    ] {
        fs::write(directory.path().join("children.pid"), "").unwrap();
        args.extend([
            "--",
            "sh",
            "-c",
            "sleep 10 & echo $! >> children.pid; echo ready >&2; wait",
        ]);
        let mut child = command(&directory)
            .args(args)
            .stderr(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        assert_eq!(
            BufReader::new(child.stderr.take().unwrap())
                .lines()
                .filter(|line| line.as_ref().unwrap() == "ready")
                .take(2)
                .count(),
            2
        );
        unsafe {
            libc::kill(child.id() as i32, libc::SIGINT);
        }
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert_eq!(status.code(), Some(130));
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "cpg ignored SIGINT"
            );
            thread::sleep(Duration::from_millis(10));
        }
        let pids = fs::read_to_string(directory.path().join("children.pid")).unwrap();
        assert_eq!(pids.lines().count(), 2);
        for pid in pids.lines() {
            let pid: i32 = pid.parse().unwrap();
            let stopped = match procfs::process::Process::new(pid).and_then(|p| p.stat()) {
                Ok(stat) => stat.state == 'Z',
                Err(procfs::ProcError::NotFound(_)) => true,
                Err(error) => panic!("{error}"),
            };
            assert!(stopped, "child {pid} is still running");
        }
    }
    assert_eq!(
        fs::read_dir(directory.path().join("interrupted"))
            .unwrap()
            .count(),
        0
    );
}

#[cfg(feature = "mock")]
#[test]
fn setup_failures_and_cleanup() {
    let directory = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("mock_service"),
        directory.path().join("mock_service"),
    )
    .unwrap();
    fs::create_dir(directory.path().join("config")).unwrap();
    let config_path = directory.path().join("config/config.toml");
    let root = directory.path().join("workspace with spaces");
    let base = format!("root = {root:?}\n");
    fs::write(
        &config_path,
        format!("{base}[setup]\nproblem = ['exit 7', 'echo should-not-run']\n"),
    )
    .unwrap();
    for (command, url, category) in [
        ("download", "https://mock.local/problems/sum", "problems"),
        (
            "prepare",
            "https://mock.local/contests/practice",
            "contests",
        ),
    ] {
        let output = run(&directory, &[command, url], 2);
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("[setup.problem] failed"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("should-not-run"));
        assert_eq!(
            fs::read_dir(root.join("mock").join(category))
                .unwrap()
                .count(),
            0
        );
    }
    for (setup, error) in [
        ("unknown = 'true'", "unknown field"),
        ("problem = ['true', 42]", "Invalid configuration"),
    ] {
        fs::write(&config_path, format!("{base}[setup]\n{setup}\n")).unwrap();
        let output = run(
            &directory,
            &["download", "https://mock.local/problems/sum"],
            2,
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains(error));
    }

    // A setup-only configuration also works without template directories.
    fs::write(
        &config_path,
        format!("{base}[setup]\nworkspace = ['printf workspace > order', 'printf second >> order']\nproblem = ['printf initialized > generated.txt', 'printf problem >> order']\nsingle_problem = ['printf single >> order']\ncontest = []\n"),
    )
    .unwrap();
    run(
        &directory,
        &["download", "https://mock.local/problems/sum"],
        0,
    );
    assert_eq!(
        fs::read(root.join("mock/problems/sum/generated.txt")).unwrap(),
        b"initialized"
    );
    assert_eq!(
        fs::read(root.join("mock/problems/sum/order")).unwrap(),
        b"workspacesecondproblemsingle"
    );

    let script = "ruby -e 'STDOUT.sync = true; puts \"setup ready\"; sleep 30'";
    fs::write(
        &config_path,
        format!("{base}[setup]\nworkspace = [{script:?}, 'echo should-not-run']\n"),
    )
    .unwrap();
    let mut child = command(&directory)
        .args(["download", "https://mock.local/problems/echo"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    assert!(
        BufReader::new(child.stderr.take().unwrap())
            .lines()
            .any(|line| line.unwrap() == "setup ready")
    );
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert_eq!(status.code(), Some(130));
            break;
        }
        if started.elapsed() > Duration::from_secs(5) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Setup ignored SIGINT");
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(!root.join("mock/problems/echo").exists());
    assert_eq!(fs::read_dir(root.join("mock/problems")).unwrap().count(), 1);
}

#[cfg(feature = "mock")]
#[test]
fn mock_submission_judging() {
    use cookie::time::{OffsetDateTime, format_description};
    let directory = tempfile::tempdir().unwrap();
    let mock = directory.path().join("mock_service");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("mock_service");
    fs::create_dir(&mock).unwrap();
    fs::copy(fixture.join("service.toml"), mock.join("service.toml")).unwrap();
    std::os::unix::fs::symlink(fixture.join("problems"), mock.join("problems")).unwrap();
    fs::create_dir(directory.path().join("config")).unwrap();
    fs::write(
        directory.path().join("config/config.toml"),
        "root = '~/workspace'\n",
    )
    .unwrap();
    fs::create_dir(directory.path().join("cookies")).unwrap();
    fs::copy(
        fixture.join("cookies.txt"),
        directory.path().join("cookies/mock.txt"),
    )
    .unwrap();
    run(
        &directory,
        &["download", "https://mock.local/problems/sum"],
        0,
    );
    let problem = directory.path().join("workspace/mock/problems/sum");
    let calls = directory.path().join("judge-calls");
    let accepted = format!(
        "File.open({:?}, 'a') {{ |f| f.puts 'run' }}; warn 'judge stderr'; puts STDIN.read.split.map(&:to_i).sum",
        calls.to_str().unwrap()
    );
    let mut submissions = Vec::new();
    for (language, source, verdict) in [
        ("ruby", accepted.as_str(), "AC"),
        ("ruby", "puts STDIN.read.include?('-') ? 0 : 3", "WA"),
        ("ruby", "abort 'runtime error'", "RE"),
        ("ruby", "sleep 30", "TLE"),
        (
            "cpp",
            "#include <iostream>\nint main() { int a,b; std::cin >> a >> b; std::cout << a+b << '\\n'; }",
            "AC",
        ),
        ("cpp", "this is not C++", "CE"),
    ] {
        fs::write(problem.join("solution"), source).unwrap();
        let output = run(
            &directory,
            &[
                "submit",
                problem.join("solution").to_str().unwrap(),
                "--language",
                language,
            ],
            0,
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        let id = stdout
            .split_whitespace()
            .nth(1)
            .unwrap()
            .trim_end_matches(':');
        let path = mock.join("submissions").join(format!("{id}.toml"));
        let stored: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(stored["status"].as_str().unwrap(), "WJ");
        let timestamp = OffsetDateTime::parse(stored["submitted_at"].as_str().unwrap(), &format_description::parse_borrowed::<2>(
            "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour sign:mandatory]:[offset_minute]"
        ).unwrap()).unwrap();
        assert_eq!(
            timestamp.unix_timestamp() as i128,
            id.parse::<i128>().unwrap() / 1_000_000_000
        );
        submissions.push((path, verdict));
    }
    assert!(
        !calls.exists(),
        "Submissions were judged before fetching results"
    );
    thread::sleep(Duration::from_millis(5100));
    // Concurrent fetches must share the persisted results instead of judging twice.
    let fetch = || {
        command(&directory)
            .current_dir(&problem)
            .arg("results")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    };
    let first = fetch();
    let second = fetch();
    let first = first.wait_with_output().unwrap();
    let second = second.wait_with_output().unwrap();
    for output in [&first, &second] {
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!String::from_utf8_lossy(&output.stderr).contains("judge stderr"));
        assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 7);
    }
    assert_eq!(first.stdout, second.stdout);
    assert!(
        calls.exists(),
        "{}\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(fs::read_to_string(&calls).unwrap(), "run\nrun\n");
    let mut cached = Vec::new();
    for (path, verdict) in submissions {
        let contents = fs::read_to_string(&path).unwrap();
        let stored: toml::Value = toml::from_str(&contents).unwrap();
        assert_eq!(stored["status"].as_str().unwrap(), verdict);
        let milliseconds: u128 = stored["time"]
            .as_str()
            .unwrap()
            .strip_suffix(" ms")
            .unwrap()
            .parse()
            .unwrap();
        if verdict == "TLE" {
            assert!(milliseconds >= 2000);
        }
        cached.push((path, contents));
    }
    let output = command(&directory)
        .current_dir(&problem)
        .arg("results")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(first.stdout, output.stdout);
    assert_eq!(fs::read_to_string(&calls).unwrap(), "run\nrun\n");
    for (path, contents) in cached {
        assert_eq!(fs::read_to_string(path).unwrap(), contents);
    }
}

#[cfg(feature = "mock")]
#[test]
fn mock_service_workflow() {
    use std::{
        os::unix::fs::{PermissionsExt, symlink},
        path::Path,
    };
    fn copy(source: &Path, target: &Path) {
        if source.is_dir() {
            fs::create_dir_all(target).unwrap();
            for entry in fs::read_dir(source).unwrap() {
                let entry = entry.unwrap();
                copy(&entry.path(), &target.join(entry.file_name()));
            }
        } else {
            fs::copy(source, target).unwrap();
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("mock_service");
    let mock = directory.path().join("mock_service");
    fs::create_dir(&mock).unwrap();
    for name in ["service.toml", "cookies.txt", "problems", "contests"] {
        copy(&fixture.join(name), &mock.join(name));
    }
    fs::create_dir(directory.path().join("config")).unwrap();
    let root = directory.path().join("workspace");
    fs::write(
        directory.path().join("config/config.toml"),
        format!(
            r#"
root = {:?}
[setup]
workspace = "ruby setup.rb"
problem = "ruby setup.rb"
contest = "ruby setup.rb"
single_problem = "ruby setup.rb"
[language.ruby]
extensions = ["rb"]
run = "ruby {{input}}"
[language.ruby.submit]
mock = "ruby"
"#,
            "~/workspace"
        ),
    )
    .unwrap();
    for (template, marker) in [
        ("workspace_template", "workspace"),
        ("problem_template", "problem"),
        ("single_problem_template", "single"),
        ("contest_template", "contest"),
    ] {
        let path = directory.path().join("config").join(template);
        fs::create_dir(&path).unwrap();
        fs::write(path.join("marker"), marker).unwrap();
        fs::write(path.join(format!("{marker}.txt")), marker).unwrap();
        fs::write(
            path.join("setup.rb"),
            "File.open('setup.log', 'a') { |file| file.puts File.read('marker') }\nFile.write(\"metadata-#{File.read('marker')}.toml\", File.read('.cpg.toml'))\nFile.write('generated.rb', 'abc')\nputs 'setup stdout'\nwarn 'setup stderr'\n",
        )
        .unwrap();
    }
    fs::write(
        directory.path().join("config/problem_template/solution.rb"),
        "# base template\n",
    )
    .unwrap();
    fs::write(
        directory
            .path()
            .join("config/single_problem_template/solution.rb"),
        "print STDIN.read",
    )
    .unwrap();
    fs::create_dir(directory.path().join("config/problem_template/src")).unwrap();
    fs::write(
        directory
            .path()
            .join("config/problem_template/src/nested.rb"),
        "abc",
    )
    .unwrap();
    let output = run(&directory, &["d", "https://mock.local/problems/echo"], 0);
    let echo = root.join("mock/problems/echo");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{}\n", echo.display())
    );
    let logs = String::from_utf8_lossy(&output.stderr);
    assert!(logs.contains("setup stdout"));
    assert!(logs.contains("setup stderr"));
    assert!(logs.contains("Missing cookies for mock:"));
    assert_eq!(logs.matches("Missing cookies").count(), 1, "{logs}");
    assert_eq!(
        fs::read_to_string(echo.join("setup.log")).unwrap(),
        "workspace\nproblem\nsingle\n"
    );
    assert_eq!(fs::read_to_string(echo.join("marker")).unwrap(), "single");
    assert_eq!(fs::read(echo.join("test/sample-1.in")).unwrap(), b"hello\n");
    assert_eq!(
        fs::read(echo.join("test/sample-1.out")).unwrap(),
        b"hello\n"
    );
    assert_eq!(fs::read_dir(echo.join("test")).unwrap().count(), 2);
    assert!(echo.join("workspace.txt").is_file());
    let metadata: toml::Value =
        toml::from_str(&fs::read_to_string(echo.join(".cpg.toml")).unwrap()).unwrap();
    assert_eq!(
        metadata["template_checksums"]["generated.rb"],
        metadata["template_checksums"]["src/nested.rb"]
    );
    assert_eq!(
        metadata["template_checksums"]["src/nested.rb"]
            .as_str()
            .unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert!(
        metadata["template_checksums"]
            .get("test/sample-1.in")
            .is_none()
    );
    assert!(metadata["template_checksums"].get(".cpg.toml").is_none());
    run(
        &directory,
        &["test", echo.join("solution.rb").to_str().unwrap()],
        0,
    );
    run(&directory, &["d", "https://mock.local/problems/echo"], 2);
    assert!(echo.join("solution.rb").is_file());
    let output = run(
        &directory,
        &["prepare", "https://mock.local/contests/practice"],
        0,
    );
    let logs = String::from_utf8_lossy(&output.stderr);
    assert!(logs.contains("i) [cpg::workspace] <download{"));
    assert_eq!(logs.matches("Missing cookies").count(), 1, "{logs}");
    assert!(!logs.contains('\u{1b}'));
    let contest = root.join("mock/contests/practice");
    assert_eq!(
        fs::read_to_string(contest.join("setup.log")).unwrap(),
        "workspace\ncontest\n"
    );
    for problem in ["1_sum", "2_echo"] {
        assert_eq!(
            fs::read_to_string(contest.join(problem).join("setup.log")).unwrap(),
            "problem\n"
        );
    }
    for (path, setups) in [
        (echo.clone(), &["workspace", "problem", "single"][..]),
        (contest.clone(), &["workspace", "contest"][..]),
        (contest.join("1_sum"), &["problem"][..]),
        (contest.join("2_echo"), &["problem"][..]),
    ] {
        let mut metadata: toml::Value =
            toml::from_str(&fs::read_to_string(path.join(".cpg.toml")).unwrap()).unwrap();
        metadata
            .as_table_mut()
            .unwrap()
            .remove("template_checksums");
        for setup in setups {
            let snapshot = path.join(format!("metadata-{setup}.toml"));
            let during_setup: toml::Value =
                toml::from_str(&fs::read_to_string(&snapshot).unwrap()).unwrap();
            assert_eq!(during_setup, metadata, "{}", snapshot.display());
        }
    }
    assert_eq!(
        fs::read_to_string(contest.join("marker")).unwrap(),
        "contest"
    );
    assert_eq!(
        fs::read_to_string(contest.join("1_sum/marker")).unwrap(),
        "problem"
    );
    assert!(!contest.join("1_sum/workspace.txt").exists());
    assert!(contest.join("1_sum/test/sample-2.out").is_file());
    assert!(contest.join("2_echo/.cpg.toml").is_file());
    assert_eq!(
        fs::read(contest.join("2_echo/test/sample-1.in")).unwrap(),
        b"hello\n"
    );
    assert_eq!(
        fs::read(contest.join("2_echo/test/sample-1.out")).unwrap(),
        b"hello\n"
    );
    assert_eq!(
        fs::read_dir(contest.join("2_echo/test")).unwrap().count(),
        2
    );
    let browser_bin = directory.path().join("browser-bin");
    fs::create_dir(&browser_bin).unwrap();
    let opener = browser_bin.join("xdg-open");
    let ruby = Command::new("ruby")
        .args(["-rrbconfig", "-e", "print RbConfig.ruby"])
        .output()
        .unwrap();
    assert!(ruby.status.success());
    fs::write(&opener, format!("#!{}\nraise 'Expected one URL' unless ARGV.length == 1\nFile.open(ENV.fetch('CPG_OPEN_LOG'), 'a') {{ |f| f.puts ARGV.fetch(0) }}\nexit Integer(ENV.fetch('CPG_OPEN_EXIT'))\n", String::from_utf8(ruby.stdout).unwrap())).unwrap();
    fs::set_permissions(&opener, fs::Permissions::from_mode(0o755)).unwrap();
    let opened = directory.path().join("opened-urls");
    let open_command = |cwd: &std::path::Path, alias: &str| {
        let mut cmd = command(&directory);
        cmd.current_dir(cwd)
            .arg(alias)
            .env("PATH", &browser_bin)
            .env("CPG_OPEN_LOG", &opened)
            .env("CPG_OPEN_EXIT", "0");
        cmd
    };
    for (cwd, alias) in [
        (echo.clone(), "open"),
        (echo.join("src"), "o"),
        (contest.join("1_sum/src"), "open"),
        (contest.clone(), "o"),
    ] {
        let output = open_command(&cwd, alias).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let expected = "https://mock.local/problems/echo\nhttps://mock.local/problems/echo\nhttps://mock.local/problems/sum\nhttps://mock.local/contests/practice\n";
    assert_eq!(fs::read_to_string(&opened).unwrap(), expected);
    let output = open_command(directory.path(), "o").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("No .cpg.toml"));
    assert_eq!(fs::read_to_string(&opened).unwrap(), expected);
    let output = open_command(&echo, "o")
        .env("CPG_OPEN_EXIT", "7")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Cannot open"));
    fs::remove_file(opener).unwrap();
    let output = open_command(&echo, "o")
        .env("PATH", &browser_bin)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Cannot open"));
    for (mode, expected) in [
        (None, vec!["mock/contests/practice", "mock/problems/echo"]),
        (
            Some("--workspace"),
            vec!["mock/contests/practice", "mock/problems/echo"],
        ),
        (Some("--contests"), vec!["mock/contests/practice"]),
        (Some("--problems"), vec!["mock/problems/echo"]),
        (
            Some("--all-problems"),
            vec![
                "mock/contests/practice/1_sum",
                "mock/contests/practice/2_echo",
                "mock/problems/echo",
            ],
        ),
    ] {
        let mut args = vec!["list"];
        if let Some(mode) = mode {
            args.push(mode);
        }
        let output = run(&directory, &args, 0);
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            format!("{}\n", expected.join("\n"))
        );
    }
    let output = run(&directory, &["config", "--root"], 0);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{}\n", root.display())
    );
    let modes = ["--workspace", "--contests", "--problems", "--all-problems"];
    for (index, mode) in modes.iter().enumerate() {
        for other in &modes[index + 1..] {
            run(&directory, &["list", mode, other], 2);
        }
    }
    run(
        &directory,
        &["d", "https://mock.local/contests/practice"],
        2,
    );
    run(&directory, &["p", "https://mock.local/problems/echo"], 2);
    run(&directory, &["d", "https://example.com/problems/echo"], 2);
    run(&directory, &["d", "https://mock.local/problems/missing"], 2);
    assert!(!root.join("mock/problems/missing").exists());
    // Failure partway through a contest must remove all staged files.
    fs::create_dir(mock.join("contests/broken")).unwrap();
    fs::write(
        mock.join("contests/broken/contest.toml"),
        "title = 'Broken'\nproblems = ['echo', 'missing']\n",
    )
    .unwrap();
    run(&directory, &["p", "https://mock.local/contests/broken"], 2);
    assert_eq!(fs::read_dir(root.join("mock/contests")).unwrap().count(), 1);
    symlink(
        "/tmp",
        directory.path().join("config/problem_template/link"),
    )
    .unwrap();
    run(&directory, &["d", "https://mock.local/problems/sum"], 2);
    fs::remove_file(directory.path().join("config/problem_template/link")).unwrap();
    assert!(!root.join("mock/problems/sum").exists());
    let solution = echo.join("solution.rb");
    let source = solution.to_str().unwrap();
    let output = run(&directory, &["s", source], 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("!) [cpg] "));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--allow-submit-unchanged-solution"));
    run(
        &directory,
        &["s", source, "--problem", "https://mock.local/problems/echo"],
        2,
    );
    let nested = echo.join("src/nested.rb");
    let output = run(&directory, &["s", nested.to_str().unwrap()], 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("unchanged from its template"));
    assert!(!mock.join("submissions").exists());
    run(
        &directory,
        &[
            "login",
            "mock",
            "--cookie-file",
            "~/mock_service/cookies.txt",
        ],
        0,
    );
    let saved = directory.path().join("cookies/mock.txt");
    assert_eq!(
        fs::metadata(&saved).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(saved.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let original = fs::read(&saved).unwrap();
    for raw in [
        "invalid",
        "mock.local\tFALSE\t/\tTRUE\t1\tsession\tmock-session\n",
        "wrong.local\tFALSE\t/\tTRUE\t0\tsession\tmock-session\n",
    ] {
        fs::write(directory.path().join("bad-cookies.txt"), raw).unwrap();
        run(
            &directory,
            &["login", "mock", "--cookie-file", "bad-cookies.txt"],
            2,
        );
        assert_eq!(fs::read(&saved).unwrap(), original);
    }
    // Leading-dot HttpOnly cookies use the same parser and jar as real services.
    fs::write(
        directory.path().join("domain-cookies.txt"),
        "#HttpOnly_.mock.local\tTRUE\t/\tTRUE\t0\tsession\tmock-session\n",
    )
    .unwrap();
    run(
        &directory,
        &["login", "mock", "--cookie-file", "domain-cookies.txt"],
        0,
    );
    run(
        &directory,
        &[
            "s",
            source,
            "--language",
            "invalid",
            "--allow-submit-unchanged-solution",
        ],
        2,
    );
    fs::write(directory.path().join("source.unknown"), "source").unwrap();
    let output = run(
        &directory,
        &[
            "s",
            "source.unknown",
            "--problem",
            "https://mock.local/problems/echo",
        ],
        2,
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("ruby\tRuby"));
    let output = run(
        &directory,
        &[
            "s",
            "~/workspace/mock/problems/echo/solution.rb",
            "--allow-submit-unchanged-solution",
        ],
        0,
    );
    let logs = String::from_utf8_lossy(&output.stderr);
    assert!(logs.contains("unchanged from its template"));
    assert!(logs.contains("Submission target: https://mock.local/problems/echo"));
    assert!(logs.contains("Submitting with language ID ruby (Ruby, 16 bytes)..."));
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("Submitted "));
    let stored_path = fs::read_dir(mock.join("submissions"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut stored: toml::Value =
        toml::from_str(&fs::read_to_string(&stored_path).unwrap()).unwrap();
    assert_eq!(stored["source"].as_str().unwrap(), "print STDIN.read");
    assert_eq!(stored["language"].as_str().unwrap(), "Ruby");
    let id = stored["id"].as_str().unwrap().to_owned();
    let output = command(&directory)
        .current_dir(&echo)
        .arg("results")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("\tWJ\t"));
    stored["status"] = "AC".into();
    stored["time"] = "1 ms".into();
    fs::write(&stored_path, toml::to_string(&stored).unwrap()).unwrap();
    // A browser submission is just another server record, including submissions from other users.
    stored["id"] = "1".into();
    stored["url"] = "https://mock.local/submissions/1".into();
    stored["submitted_at"] = "00000000000000000001".into();
    fs::write(
        mock.join("submissions/1.toml"),
        toml::to_string(&stored).unwrap(),
    )
    .unwrap();
    stored["id"] = "2".into();
    stored["url"] = "https://mock.local/submissions/2".into();
    stored["user"] = "someone-else".into();
    fs::write(
        mock.join("submissions/2.toml"),
        toml::to_string(&stored).unwrap(),
    )
    .unwrap();
    let output = command(&directory)
        .current_dir(&contest)
        .arg("results")
        .output()
        .unwrap();
    assert!(output.status.success());
    let results = String::from_utf8_lossy(&output.stdout);
    assert_eq!(results.lines().count(), 3);
    assert!(results.contains(&id) && results.contains("\tAC\t1 ms\t"));
    let output = command(&directory)
        .current_dir(&echo)
        .args(["r", "--limit", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 2);
    let ui = command(&directory)
        .current_dir(&echo)
        .args(["r", "--ui", "--limit", "1"])
        .output()
        .unwrap();
    assert!(ui.status.success());
    assert_eq!(ui.stdout, output.stdout);
    assert!(!ui.stdout.contains(&0x1b));
    let terminal_test = Command::new("ruby")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/results_ui.rb"))
        .arg(env!("CARGO_BIN_EXE_cpg"))
        .arg(&stored_path)
        .envs(
            command(&directory)
                .get_envs()
                .map(|(key, value)| (key, value.unwrap())),
        )
        .current_dir(&echo)
        .output()
        .unwrap();
    assert!(
        terminal_test.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&terminal_test.stdout),
        String::from_utf8_lossy(&terminal_test.stderr)
    );
    // Template edits after download do not change the saved baseline; editing the solution allows submission.
    fs::write(
        directory
            .path()
            .join("config/single_problem_template/solution.rb"),
        "# new template\n",
    )
    .unwrap();
    run(&directory, &["s", source], 2);
    fs::write(&solution, "print STDIN.read\n").unwrap();
    run(&directory, &["s", source], 0);
    assert_eq!(fs::read_dir(mock.join("submissions")).unwrap().count(), 4);
    fs::write(&nested, "puts 42").unwrap();
    run(&directory, &["s", nested.to_str().unwrap()], 0);

    let config_path = directory.path().join("config/config.toml");
    let mut config: toml::Value =
        toml::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    config["language"]["ruby"]
        .as_table_mut()
        .unwrap()
        .insert("preprocess".into(), "ruby ~/preprocess.rb {input}".into());
    config["language"]["ruby"]
        .as_table_mut()
        .unwrap()
        .insert("presubmit".into(), "ruby ~/presubmit.rb {input}".into());
    fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();
    fs::write(directory.path().join("preprocess.rb"), "abort unless File.read('marker') == 'single'; File.open('order', 'a') { |f| f.puts 'preprocess' }; puts '# preprocess'; puts File.read(ARGV.fetch(0))").unwrap();
    fs::write(directory.path().join("presubmit.rb"), "source = File.read(ARGV.fetch(0)); abort unless source.start_with?(\"# preprocess\\n\") && STDIN.read == source; File.open('order', 'a') { |f| f.puts 'presubmit' }; print source; puts '# presubmit'").unwrap();
    fs::write(&solution, "print STDIN.read").unwrap();
    run(&directory, &["s", source], 2);
    assert!(!echo.join("order").exists());
    let output = run(
        &directory,
        &["s", source, "--allow-submit-unchanged-solution"],
        0,
    );
    assert_eq!(
        fs::read_to_string(echo.join("order")).unwrap(),
        "preprocess\npresubmit\n"
    );
    assert_eq!(fs::read_to_string(&solution).unwrap(), "print STDIN.read");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let id = stdout
        .split_whitespace()
        .nth(1)
        .unwrap()
        .trim_end_matches(':');
    let latest = mock.join("submissions").join(format!("{id}.toml"));
    let submitted: toml::Value = toml::from_str(&fs::read_to_string(latest).unwrap()).unwrap();
    assert_eq!(
        submitted["source"].as_str().unwrap(),
        "# preprocess\nprint STDIN.read\n# presubmit\n"
    );
    let before = fs::read_dir(mock.join("submissions")).unwrap().count();
    for (stage, command) in [
        ("presubmit", "printf partial; exit 1"),
        ("presubmit", "true"),
        ("preprocess", "false"),
    ] {
        config["language"]["ruby"][stage] = command.into();
        fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();
        run(
            &directory,
            &["s", source, "--allow-submit-unchanged-solution"],
            2,
        );
        assert_eq!(
            fs::read_dir(mock.join("submissions")).unwrap().count(),
            before
        );
        assert!(fs::read_dir(&echo).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("cpg_preprocessed_")
        }));
    }
    let settings = fs::read_to_string(mock.join("service.toml"))
        .unwrap()
        .replace("mock-session", "expired-session");
    fs::write(mock.join("service.toml"), settings).unwrap();
    let output = command(&directory)
        .current_dir(&echo)
        .arg("results")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}
