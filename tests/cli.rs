use std::{
    fs,
    io::{BufRead, BufReader, Write},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

fn command(directory: &TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cpcli"));
    command
        .current_dir(directory.path())
        .env("HOME", directory.path())
        .env("CARGO_MANIFEST_DIR", directory.path())
        .env("CPCLI_CONFIG_HOME", "~/config")
        .env("CPCLI_COOKIES_HOME", "~/cookies");
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
    fs::write(directory.path().join("test/sample.in"), input).unwrap();
    fs::write(directory.path().join("test/sample.out"), output).unwrap();
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
        ("\n", "cpcli"),
        ("~/my \"競プロ\" workspace\n", "my \"競プロ\" workspace"),
        ("relative workspace\n", "relative workspace"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let output = initialize(&directory, input, 0);
        assert!(String::from_utf8_lossy(&output.stderr).contains("Workspace root [~/cpcli]:"));
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
            "cpcli login",
            "cpcli download",
            "cpcli prepare",
            "cpcli test",
            "cpcli submit",
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
    assert!(!directory.path().join("cpcli").exists());
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

    let cpcli_config = directory.path().join("config/config.toml");
    let original_cpcli = fs::read(&cpcli_config).unwrap();
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
    assert_eq!(fs::read(cpcli_config).unwrap(), original_cpcli);

    run_with_input(
        command(&directory)
            .arg("init")
            .env("XDG_CONFIG_HOME", "~/oj-config")
            .env("CPCLI_CONFIG_HOME", "~/declined"),
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
                .env("CPCLI_CONFIG_HOME", "~/invalid"),
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
fn cli_contract_and_local_judging() {
    let directory = tempfile::tempdir().unwrap();
    run(&directory, &["--version"], 0);
    for name in [
        "init", "login", "download", "d", "prepare", "p", "test", "t", "generate", "g", "submit",
        "s", "results", "r", "list", "open", "o",
    ] {
        run(&directory, &[name, "--help"], 0);
    }
    let help = run(&directory, &["results", "--help"], 0);
    assert!(String::from_utf8_lossy(&help.stdout).contains("--ui"));
    run(&directory, &["results", "--watch"], 2);
    for options in [
        vec!["--time-limit", "0"],
        vec!["--memory-limit", "0"],
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
    run(&directory, &["t", "--", "cat"], 0);
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
            .any(|line| line.starts_with("x) [cpcli] "))
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("1/1 accepted"));
    fs::write(directory.path().join("test/second.in"), "").unwrap();
    fs::write(directory.path().join("test/second.out"), "wrong").unwrap();
    let output = run(&directory, &["test", "--fast-fail", "--", "true"], 1);
    assert!(String::from_utf8_lossy(&output.stdout).contains("0/1 accepted"));
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
        .env("CPCLI_CONFIG_HOME", directory.path().join("config"))
        .env("CPCLI_COOKIES_HOME", directory.path().join("cookies"))
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
    run(&directory, &["test", "solution.cpp"], 0);
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
            .starts_with("cpcli_preprocessed_")
    }));
}

#[test]
fn limits_interactive_and_cleanup() {
    let directory = tempfile::tempdir().unwrap();
    case(&directory, b"", b"");
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
    run(
        &directory,
        &[
            "test",
            "--interactive",
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
    case(&directory, b"", b"");
    run(
        &directory,
        &[
            "test",
            "--interactive",
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
    run(
        &directory,
        &[
            "test",
            "--interactive",
            "--judge",
            "exit 1",
            "--time-limit",
            "100",
            "--",
            "sleep",
            "10",
        ],
        1,
    );
    let mut child = command(&directory)
        .args(["test", "--", "sh", "-c", "echo ready >&2; sleep 10"])
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    assert!(
        BufReader::new(child.stderr.take().unwrap())
            .lines()
            .any(|line| line.unwrap() == "ready")
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
            "cpcli ignored SIGINT"
        );
        thread::sleep(Duration::from_millis(10));
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
    run(&directory, &["d", "https://mock.local/problems/echo"], 0);
    let echo = root.join("mock/problems/echo");
    assert_eq!(fs::read_to_string(echo.join("marker")).unwrap(), "single");
    assert_eq!(fs::read(echo.join("test/sample.in")).unwrap(), b"hello\n");
    assert!(echo.join("workspace.txt").is_file());
    let metadata: toml::Value =
        toml::from_str(&fs::read_to_string(echo.join(".cpcli.toml")).unwrap()).unwrap();
    assert_eq!(
        metadata["template_checksums"]["src/nested.rb"]
            .as_str()
            .unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert!(
        metadata["template_checksums"]
            .get("test/sample.in")
            .is_none()
    );
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
    assert!(logs.contains("i) [cpcli::workspace] <download{"));
    assert!(!logs.contains('\u{1b}'));
    let contest = root.join("mock/contests/practice");
    assert_eq!(
        fs::read_to_string(contest.join("marker")).unwrap(),
        "contest"
    );
    assert_eq!(
        fs::read_to_string(contest.join("01_sum/marker")).unwrap(),
        "problem"
    );
    assert!(!contest.join("01_sum/workspace.txt").exists());
    assert!(contest.join("01_sum/test/sample-2.out").is_file());
    assert!(contest.join("02_echo/.cpcli.toml").is_file());
    let browser_bin = directory.path().join("browser-bin");
    fs::create_dir(&browser_bin).unwrap();
    let opener = browser_bin.join("xdg-open");
    let ruby = Command::new("ruby")
        .args(["-rrbconfig", "-e", "print RbConfig.ruby"])
        .output()
        .unwrap();
    assert!(ruby.status.success());
    fs::write(&opener, format!("#!{}\nraise 'Expected one URL' unless ARGV.length == 1\nFile.open(ENV.fetch('CPCLI_OPEN_LOG'), 'a') {{ |f| f.puts ARGV.fetch(0) }}\nexit Integer(ENV.fetch('CPCLI_OPEN_EXIT'))\n", String::from_utf8(ruby.stdout).unwrap())).unwrap();
    fs::set_permissions(&opener, fs::Permissions::from_mode(0o755)).unwrap();
    let opened = directory.path().join("opened-urls");
    let open_command = |cwd: &std::path::Path, alias: &str| {
        let mut cmd = command(&directory);
        cmd.current_dir(cwd)
            .arg(alias)
            .env("PATH", &browser_bin)
            .env("CPCLI_OPEN_LOG", &opened)
            .env("CPCLI_OPEN_EXIT", "0");
        cmd
    };
    for (cwd, alias) in [
        (echo.clone(), "open"),
        (echo.join("src"), "o"),
        (contest.join("01_sum/src"), "open"),
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
    assert!(String::from_utf8_lossy(&output.stderr).contains("No .cpcli.toml"));
    assert_eq!(fs::read_to_string(&opened).unwrap(), expected);
    let output = open_command(&echo, "o")
        .env("CPCLI_OPEN_EXIT", "7")
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
                "mock/contests/practice/01_sum",
                "mock/contests/practice/02_echo",
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
        args.push("--path");
        let output = run(&directory, &args, 0);
        let absolute: Vec<_> = expected
            .iter()
            .map(|path| root.join(path).display().to_string())
            .collect();
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            format!("{}\n", absolute.join("\n"))
        );
    }
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
    assert!(String::from_utf8_lossy(&output.stderr).contains("!) [cpcli] "));
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
        .arg(env!("CARGO_BIN_EXE_cpcli"))
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
    run(
        &directory,
        &["s", source, "--allow-submit-unchanged-solution"],
        0,
    );
    assert_eq!(
        fs::read_to_string(echo.join("order")).unwrap(),
        "preprocess\npresubmit\n"
    );
    assert_eq!(fs::read_to_string(&solution).unwrap(), "print STDIN.read");
    let latest = fs::read_dir(mock.join("submissions"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .max_by_key(|path| {
            path.file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
        })
        .unwrap();
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
                .starts_with("cpcli_preprocessed_")
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
