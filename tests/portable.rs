use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
    thread,
    time::Duration,
};
use tempfile::TempDir;

fn compiler() -> &'static Path {
    static COMPILER: OnceLock<PathBuf> = OnceLock::new();
    COMPILER.get_or_init(|| {
        let output = Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        Path::new(String::from_utf8(output.stdout).unwrap().trim())
            .join("bin")
            .join(format!("rustc{}", std::env::consts::EXE_SUFFIX))
    })
}

fn helper() -> PathBuf {
    static DIRECTORY: OnceLock<TempDir> = OnceLock::new();
    DIRECTORY
        .get_or_init(|| {
            let directory = tempfile::tempdir().unwrap();
            let output = Command::new(compiler())
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tests/fixtures/portable.rs"
                ))
                .arg("-o")
                .arg(
                    directory
                        .path()
                        .join(format!("helper{}", std::env::consts::EXE_SUFFIX)),
                )
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            directory
        })
        .path()
        .join(format!("helper{}", std::env::consts::EXE_SUFFIX))
}

fn command(directory: &TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cpg"));
    command
        .current_dir(directory.path())
        .env("HOME", directory.path())
        .env("USERPROFILE", directory.path())
        .env("CPG_CONFIG_HOME", "~/config")
        .env("CPG_COOKIES_HOME", "~/cookies")
        .env("CPG_LOG", "")
        .env("CPG_TEST_RUSTC", compiler())
        .env("CARGO_MANIFEST_DIR", directory.path())
        .env("CPG_TEST_HELPER", helper());
    command
}

fn run(command: &mut Command, code: i32) -> Output {
    let output = command.output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(code),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn directory() -> TempDir {
    let directory = tempfile::Builder::new()
        .prefix("cpg portable ' ")
        .tempdir()
        .unwrap();
    fs::create_dir(directory.path().join("config")).unwrap();
    fs::write(
        directory.path().join("config/config.toml"),
        "root = '~/workspace'\n",
    )
    .unwrap();
    fs::create_dir(directory.path().join("test")).unwrap();
    fs::write(directory.path().join("test/sample.in"), "sample\n").unwrap();
    fs::write(directory.path().join("test/sample.out"), "sample\n").unwrap();
    directory
}

#[test]
fn paths_completion_and_direct_judging() {
    let directory = directory();
    let output = run(command(&directory).args(["config", "--config-dir"]), 0);
    assert_eq!(
        Path::new(String::from_utf8(output.stdout).unwrap().trim()),
        directory.path().join("config")
    );
    for shell in ["bash", "zsh", "powershell"] {
        let output = run(command(&directory).args(["completion", shell]), 0);
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .contains("__complete_word__")
        );
    }
    run(
        command(&directory)
            .args(["test", "--"])
            .arg(helper())
            .arg("copy"),
        0,
    );
    run(
        command(&directory)
            .args(["test", "--test-dir"])
            .arg(directory.path().join("test"))
            .arg(helper()),
        0,
    );
    let special = directory.path().join("path & ! %CPG_TEST_HELPER%");
    fs::create_dir(&special).unwrap();
    let executable = special.join(format!("copy{}", std::env::consts::EXE_SUFFIX));
    fs::copy(helper(), &executable).unwrap();
    run(
        command(&directory)
            .args(["test", "--test-dir"])
            .arg(directory.path().join("test"))
            .arg(&executable),
        0,
    );
    fs::write(directory.path().join("test/sample.out"), "different\n").unwrap();
    run(
        command(&directory)
            .args(["test", "--"])
            .arg(helper())
            .arg("copy"),
        1,
    );
    run(
        command(&directory)
            .args(["test", "--"])
            .arg(helper())
            .arg("fail"),
        1,
    );
    let output = run(
        command(&directory)
            .args(["test", "--memory-limit", "64", "--time-limit", "5000", "--"])
            .arg(helper())
            .arg("memory"),
        1,
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("MLE"),
        "{output:?}"
    );
}

#[test]
fn compile_transform_generate_and_interactive() {
    let directory = directory();
    let config_path = directory.path().join("config/config.toml");
    fs::write(
        &config_path,
        r#"
root = '~/workspace'
[language.rust]
extensions = ['rs']
preprocess = 'cat {input} > {processed}'
compile = '"$CPG_TEST_RUSTC" {input} -o {binary}'
run = '{binary} copy'
"#
        .replace(
            "cat {input}",
            if cfg!(windows) {
                "type {input}"
            } else {
                "cat {input}"
            },
        )
        .replace(
            "$CPG_TEST_RUSTC",
            if cfg!(windows) {
                "%CPG_TEST_RUSTC%"
            } else {
                "$CPG_TEST_RUSTC"
            },
        ),
    )
    .unwrap();
    fs::write(
        directory.path().join("solution.rs"),
        include_str!("fixtures/portable.rs"),
    )
    .unwrap();
    run(command(&directory).args(["test", "solution.rs"]), 0);
    let config = fs::read_to_string(&config_path).unwrap();
    fs::write(&config_path, config.replace(" > {processed}", "")).unwrap();
    run(command(&directory).args(["test", "solution.rs"]), 0);
    assert_eq!(
        fs::read_to_string(directory.path().join("solution.rs")).unwrap(),
        include_str!("fixtures/portable.rs")
    );
    assert!(fs::read_dir(directory.path()).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("cpg_preprocessed_")
    }));
    let executable = format!("solution{}", std::env::consts::EXE_SUFFIX);
    assert!(directory.path().join(&executable).is_file());
    run(
        command(&directory)
            .args(["test", "--"])
            .arg(directory.path().join(executable))
            .arg("copy"),
        0,
    );
    run(
        command(&directory)
            .args(["generate", "--count", "2", "-j", "2", "--"])
            .arg(helper())
            .arg("generate"),
        0,
    );
    assert_eq!(
        fs::read_dir(directory.path().join("random"))
            .unwrap()
            .count(),
        2
    );
    run(
        command(&directory)
            .args([
                "test",
                "--interactive",
                "--judge",
                if cfg!(windows) {
                    "\"%CPG_TEST_HELPER%\" judge"
                } else {
                    "\"$CPG_TEST_HELPER\" judge"
                },
                "--time-limit",
                "5000",
                "--",
            ])
            .arg(helper())
            .arg("answer"),
        0,
    );
}

#[test]
fn process_tree_cleanup() {
    let directory = directory();
    for (mode, code) in [("spawn", 1), ("background", 0)] {
        fs::write(directory.path().join("test/sample.out"), "").unwrap();
        let marker = directory.path().join(mode);
        let output = run(
            command(&directory)
                .args(["test", "--time-limit", "500", "--"])
                .arg(helper())
                .args([mode, marker.to_str().unwrap()]),
            code,
        );
        if mode == "spawn" {
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("TLE"),
                "{output:?}"
            );
        }
        thread::sleep(Duration::from_millis(2200));
        assert!(!marker.exists(), "Descendant survived {mode}");
    }
}

#[cfg(feature = "mock")]
#[test]
fn download_login_and_submission() {
    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    let directory = directory();
    copy(
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/mock_service")),
        &directory.path().join("mock_service"),
    );
    for count in [2, 27] {
        fs::write(
            directory
                .path()
                .join("mock_service/contests/practice/contest.toml"),
            format!("title = 'Practice'\nproblems = {:?}\n", vec!["echo"; count]),
        )
        .unwrap();
        for alphabetic in [false, true] {
            fs::write(
                directory.path().join("config/config.toml"),
                format!("root = '~/workspace'\nalphabetic = {alphabetic}\n"),
            )
            .unwrap();
            let mut cmd = command(&directory);
            cmd.args(["prepare", "https://mock.local/contests/practice"]);
            run(&mut cmd, 0);
            let contest = directory.path().join("workspace/mock/contests/practice");
            let (first, last) = match (alphabetic, count) {
                (false, 2) => ("1", "2"),
                (false, _) => ("01", "27"),
                (true, 2) => ("a", "b"),
                (true, _) => ("_a", "aa"),
            };
            for prefix in [first, last] {
                assert!(
                    contest
                        .join(format!("{prefix}_echo/test/sample-1.in"))
                        .is_file()
                );
            }
            if alphabetic && count == 27 {
                assert!(contest.join("_z_echo/.cpg.toml").is_file());
            }
            fs::remove_dir_all(contest).unwrap();
        }
    }
    fs::write(
        directory.path().join("config/config.toml"),
        "root = '~/workspace'\n[setup]\nproblem = 'echo initialized>ready'\n",
    )
    .unwrap();
    fs::create_dir(directory.path().join("config/problem_template")).unwrap();
    fs::write(
        directory
            .path()
            .join("config/problem_template/solution.txt"),
        "print STDIN.read",
    )
    .unwrap();
    run(
        command(&directory).args(["prepare", "https://mock.local/problems/echo"]),
        0,
    );
    let problem = directory.path().join("workspace/mock/problems/echo");
    assert_eq!(
        fs::read_to_string(problem.join("ready")).unwrap().trim(),
        "initialized"
    );
    run(
        command(&directory).args(["prepare", "https://mock.local/problems/echo"]),
        2,
    );
    let module = directory
        .path()
        .join("modules/Microsoft.PowerShell.Security");
    fs::create_dir_all(&module).unwrap();
    fs::write(
        module.join("Microsoft.PowerShell.Security.psm1"),
        "function Set-Acl { throw 'Inherited PSModulePath must not be used' }",
    )
    .unwrap();
    run(
        command(&directory)
            .env("PSModulePath", directory.path().join("modules"))
            .args(["login", "mock", "--cookie-file", "mock_service/cookies.txt"]),
        0,
    );
    let cookies = fs::read(directory.path().join("cookies/mock.txt")).unwrap();
    let info = run(command(&directory).args(["login", "mock", "--info"]), 0);
    assert_eq!(
        info.stdout,
        b"mock-user\nhttps://mock.local/users/mock-user\n"
    );
    assert_eq!(
        fs::read(directory.path().join("cookies/mock.txt")).unwrap(),
        cookies
    );
    run(
        command(&directory)
            .args(["submit"])
            .arg(problem.join("solution.txt"))
            .args(["--language", "ruby", "--allow-submit-unchanged-solution"]),
        0,
    );
    let output = run(command(&directory).current_dir(&problem).arg("results"), 0);
    assert!(String::from_utf8_lossy(&output.stdout).contains("https://mock.local/submissions/"));
}
