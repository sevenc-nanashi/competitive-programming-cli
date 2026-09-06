use crate::{
    cli::{FloatErrorType, Generate, ProgramArgs, ShowIo, Test},
    config::{Config, Language, expand_path},
};
use anyhow::{Context, Result, ensure};
use console::Style;
use std::{
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::{self, Read, Write},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
enum Invocation {
    Shell(String),
    Direct(Vec<OsString>),
}

#[derive(Clone, Debug)]
pub struct Program {
    invocation: Invocation,
    pub cwd: PathBuf,
    prepared_source: Option<Arc<tempfile::NamedTempFile>>,
}

impl Program {
    fn shell(command: String, cwd: PathBuf) -> Self {
        Self {
            invocation: Invocation::Shell(command),
            cwd,
            prepared_source: None,
        }
    }

    pub fn prepare(config: &Config, args: &ProgramArgs, interrupted: &AtomicBool) -> Result<Self> {
        if let Some(file) = &args.file {
            let file = fs::canonicalize(expand_path(file)?)
                .with_context(|| format!("Cannot open {}", file.display()))?;
            ensure!(file.is_file(), "Not a source file: {}", file.display());
            let language = config.language(&file)?;
            let profile = args
                .profile
                .as_ref()
                .map(|name| {
                    language
                        .profile
                        .get(name)
                        .with_context(|| format!("Unknown profile: {name}"))
                })
                .transpose()?;
            let compile = profile
                .and_then(|p| p.compile.as_deref())
                .or(language.compile.as_deref());
            let run = match profile.and_then(|p| p.run.as_deref()) {
                Some(run) => run,
                None => &language.run,
            };
            let cwd = file
                .parent()
                .context("Source file has no parent")?
                .to_owned();
            let binary = file.with_extension("");
            ensure!(
                compile.is_none() || binary != file,
                "Compiled output would overwrite the source file"
            );
            let prepared_source = prepare_source(language, &file, false, interrupted)?;
            let input = match &prepared_source {
                Some(source) => source.path(),
                None => &file,
            };
            let expand = |command: &str| -> Result<String> {
                Ok(command
                    .replace("{input}", &quote(input.as_os_str())?)
                    .replace("{binary}", &quote(binary.as_os_str())?))
            };
            if let Some(compile) = compile {
                let command = expand(compile)?;
                tracing::info!("Compiling {}: {command}", file.display());
                let program = Self::shell(command, cwd.clone());
                let result = execute(
                    &program,
                    Stdio::null(),
                    Stdio::inherit(),
                    Limits::default(),
                    interrupted,
                )?;
                ensure!(
                    result.verdict == Verdict::Ac,
                    "Compilation failed ({})",
                    result.verdict
                );
            }
            let mut program = Self::shell(expand(run)?, cwd);
            program.prepared_source = prepared_source;
            Ok(program)
        } else {
            ensure!(
                !args.command.is_empty(),
                "Specify a source file or a command after --"
            );
            ensure!(args.profile.is_none(), "--profile requires a source file");
            Ok(Self {
                invocation: Invocation::Direct(args.command.clone()),
                cwd: std::env::current_dir()?,
                prepared_source: None,
            })
        }
    }
}

fn quote(value: &OsStr) -> Result<String> {
    let value = value.to_str().context(
        "Shell placeholders require UTF-8 paths; use a direct command for non-UTF-8 paths",
    )?;
    Ok(shlex::try_quote(value)?.into_owned())
}

pub fn prepare_source(
    language: &Language,
    input: &Path,
    for_submission: bool,
    interrupted: &AtomicBool,
) -> Result<Option<Arc<tempfile::NamedTempFile>>> {
    let mut output: Option<Arc<tempfile::NamedTempFile>> = None;
    let stages = [
        ("preprocess", language.preprocess.as_deref()),
        (
            "presubmit",
            if for_submission {
                language.presubmit.as_deref()
            } else {
                None
            },
        ),
    ];
    for (stage, command) in stages {
        if let Some(command) = command {
            let input = match &output {
                Some(source) => source.path(),
                None => input,
            };
            output = Some(Arc::new(transform_source(
                stage,
                command,
                input,
                interrupted,
            )?));
        }
    }
    Ok(output)
}

fn transform_source(
    stage: &str,
    command: &str,
    input: &Path,
    interrupted: &AtomicBool,
) -> Result<tempfile::NamedTempFile> {
    let cwd = input
        .parent()
        .context("Source file has no parent")?
        .to_owned();
    let suffix = match input.extension() {
        Some(extension) => format!(
            ".{}",
            extension
                .to_str()
                .context("Source extension must be UTF-8")?
        ),
        None => String::new(),
    };
    // Keep the extension and parent directory for compilers and relative includes.
    let output = tempfile::Builder::new()
        .prefix("cpg_preprocessed_")
        .suffix(&suffix)
        .tempfile_in(&cwd)?;
    let program = Program::shell(command.replace("{input}", &quote(input.as_os_str())?), cwd);
    tracing::info!("Running {stage} for {}", input.display());
    let result = execute(
        &program,
        File::open(input)?.into(),
        output.reopen()?.into(),
        Limits::default(),
        interrupted,
    )?;
    ensure!(
        result.verdict == Verdict::Ac,
        "{stage} failed ({})",
        result.verdict
    );
    let source = fs::read_to_string(output.path())
        .with_context(|| format!("{stage} output must be UTF-8"))?;
    ensure!(
        !source.trim().is_empty(),
        "{stage} produced empty output; configure it to write source code to stdout"
    );
    Ok(output)
}

struct ManagedChild {
    child: Child,
    status: Option<ExitStatus>,
}

impl ManagedChild {
    fn spawn(program: &Program, stdin: Stdio, stdout: Stdio) -> Result<Self> {
        let mut command = match &program.invocation {
            Invocation::Shell(script) => {
                let mut command = Command::new("sh");
                command.args(["-c", script]);
                command
            }
            Invocation::Direct(argv) => {
                let (executable, args) = argv.split_first().context("Empty command")?;
                let mut command = Command::new(executable);
                command.args(args);
                command
            }
        };
        let executable = command.get_program().to_owned();
        let child = command
            .current_dir(&program.cwd)
            .stdin(stdin)
            .stdout(stdout)
            .stderr(Stdio::inherit())
            .process_group(0)
            .spawn()
            .with_context(|| format!("Cannot start {}", executable.to_string_lossy()))?;
        Ok(Self {
            child,
            status: None,
        })
    }

    fn poll(&mut self) -> Result<Option<ExitStatus>> {
        if self.status.is_none() {
            self.status = self.child.try_wait()?;
        }
        Ok(self.status)
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        // The shell and all children inheriting its process group are owned by this run.
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGKILL);
        }
        let _ = self.child.wait();
    }
}

#[derive(Clone, Copy, Default)]
struct Limits {
    time: Option<Duration>,
    memory: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Ac,
    Wa,
    Re,
    Tle,
    Mle,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Ac => "AC",
            Self::Wa => "WA",
            Self::Re => "RE",
            Self::Tle => "TLE",
            Self::Mle => "MLE",
        })
    }
}

struct RunResult {
    verdict: Verdict,
    elapsed: Duration,
    memory: u64,
}

fn memory_usage(group: u32) -> Result<u64> {
    let mut memory = 0;
    // ponytail: /proc polling misses brief peaks and scans all processes; use delegated cgroups for strict accounting.
    for process in procfs::process::all_processes()? {
        // Processes may disappear between enumeration and reading stat; other users may hide theirs.
        let stat = match process.and_then(|p| p.stat()) {
            Ok(stat) => stat,
            Err(procfs::ProcError::NotFound(_)) | Err(procfs::ProcError::PermissionDenied(_)) => {
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        if stat.pgrp == group as i32 {
            memory += stat.rss * procfs::page_size();
        }
    }
    Ok(memory)
}

fn monitor(
    children: &mut [&mut ManagedChild],
    limits: Limits,
    interrupted: &AtomicBool,
) -> Result<RunResult> {
    let started = Instant::now();
    let mut peak = 0;
    let verdict = loop {
        ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
        peak = peak.max(memory_usage(children[0].child.id())?);
        if limits.memory.is_some_and(|limit| peak > limit) {
            break Verdict::Mle;
        }
        let mut finished = true;
        let mut failure = None;
        for (i, child) in children.iter_mut().enumerate() {
            match child.poll()? {
                Some(status) if !status.success() => {
                    failure = Some(if i == 0 { Verdict::Re } else { Verdict::Wa });
                    break;
                }
                Some(_) => (),
                None => finished = false,
            }
        }
        if let Some(failure) = failure {
            break failure;
        }
        if finished {
            break Verdict::Ac;
        }
        if limits.time.is_some_and(|limit| started.elapsed() >= limit) {
            break Verdict::Tle;
        }
        thread::sleep(Duration::from_millis(10));
    };
    Ok(RunResult {
        verdict,
        elapsed: started.elapsed(),
        memory: peak,
    })
}

fn execute(
    program: &Program,
    input: Stdio,
    output: Stdio,
    limits: Limits,
    interrupted: &AtomicBool,
) -> Result<RunResult> {
    ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
    let mut child = ManagedChild::spawn(program, input, output)?;
    monitor(&mut [&mut child], limits, interrupted)
}

pub fn setup(command: &str, directory: &Path, interrupted: &AtomicBool) -> Result<()> {
    let program = Program::shell(command.to_owned(), directory.to_owned());
    let result = execute(
        &program,
        Stdio::null(),
        io::stderr().into(),
        Limits::default(),
        interrupted,
    )?;
    ensure!(
        result.verdict == Verdict::Ac,
        "Command failed ({})",
        result.verdict
    );
    Ok(())
}

fn relay(
    mut input: impl Read,
    mut output: impl Write,
    prefix: &str,
    style: Style,
    transcript: Option<Arc<Mutex<File>>>,
) -> io::Result<()> {
    let mut buffer = [0; 4096];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            return Ok(());
        }
        if let Some(transcript) = &transcript {
            write!(
                transcript.lock().expect("transcript lock poisoned"),
                "{}",
                style.apply_to(format!("{prefix}{}", String::from_utf8_lossy(&buffer[..n])))
            )?;
        }
        if let Err(error) = output.write_all(&buffer[..n]).and_then(|()| output.flush()) {
            if error.kind() == io::ErrorKind::BrokenPipe {
                return Ok(());
            }
            return Err(error);
        }
    }
}

fn interactive(
    program: &Program,
    judge: &Program,
    limits: Limits,
    interrupted: &AtomicBool,
    transcript: Option<File>,
) -> Result<RunResult> {
    let mut solution = ManagedChild::spawn(program, Stdio::piped(), Stdio::piped())?;
    let mut judge = ManagedChild::spawn(judge, Stdio::piped(), Stdio::piped())?;
    let solution_out = solution.child.stdout.take().expect("piped stdout");
    let judge_in = judge.child.stdin.take().expect("piped stdin");
    let judge_out = judge.child.stdout.take().expect("piped stdout");
    let solution_in = solution.child.stdin.take().expect("piped stdin");
    let transcript = transcript.map(|file| Arc::new(Mutex::new(file)));
    let forward_transcript = transcript.clone();
    let forward = thread::spawn(move || {
        relay(
            solution_out,
            judge_in,
            "> ",
            Style::new().yellow(),
            forward_transcript,
        )
    });
    let backward = thread::spawn(move || {
        relay(
            judge_out,
            solution_in,
            "< ",
            Style::new().green(),
            transcript,
        )
    });
    let result = monitor(&mut [&mut solution, &mut judge], limits, interrupted);
    drop(solution);
    drop(judge);
    let forwarded = forward
        .join()
        .map_err(|_| anyhow::anyhow!("Solution relay panicked"))?;
    let backwarded = backward
        .join()
        .map_err(|_| anyhow::anyhow!("Judge relay panicked"))?;
    let result = result?;
    forwarded?;
    backwarded?;
    Ok(result)
}

enum Judge {
    File(Program),
    Shell(String, PathBuf),
}

impl Judge {
    fn prepare(config: &Config, command: &str, interrupted: &AtomicBool) -> Result<Self> {
        let path = expand_path(command)?;
        if path.is_file() {
            let path = fs::canonicalize(path)?;
            // A configured source uses the same compiler/profile machinery as the solution.
            if config.match_language(&path)?.is_some() {
                let args = ProgramArgs {
                    file: Some(path),
                    command: vec![],
                    profile: None,
                };
                return Ok(Self::File(Program::prepare(config, &args, interrupted)?));
            }
            return Ok(Self::File(Program {
                invocation: Invocation::Direct(vec![path.into()]),
                cwd: std::env::current_dir()?,
                prepared_source: None,
            }));
        }
        Ok(Self::Shell(command.to_owned(), std::env::current_dir()?))
    }

    fn command(
        &self,
        input: Option<&Path>,
        expected: Option<&Path>,
        actual: Option<&Path>,
    ) -> Result<Program> {
        let values = [
            ("{test_input}", input),
            ("{solution_output}", actual),
            ("{test_output}", expected),
        ];
        match self {
            Self::File(program) => {
                let mut program = program.clone();
                for (_, path) in values {
                    if let Some(path) = path {
                        match &mut program.invocation {
                            Invocation::Direct(argv) => argv.push(path.as_os_str().to_owned()),
                            Invocation::Shell(command) => {
                                command.push(' ');
                                command.push_str(&quote(path.as_os_str())?);
                            }
                        }
                    }
                }
                Ok(program)
            }
            Self::Shell(command, cwd) => {
                let explicit = values.iter().any(|(key, _)| command.contains(key));
                let mut command = command.clone();
                for (key, path) in values {
                    if command.contains(key) {
                        let path = path.with_context(|| {
                            format!("{key} is unavailable in this interactive test")
                        })?;
                        command = command.replace(key, &quote(path.as_os_str())?);
                    } else if !explicit && let Some(path) = path {
                        command.push(' ');
                        command.push_str(&quote(path.as_os_str())?);
                    }
                }
                Ok(Program::shell(command, cwd.clone()))
            }
        }
    }
}

fn inputs(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut inputs = Vec::new();
    if !directory.try_exists()? {
        return Ok(inputs);
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_file() && entry.path().extension() == Some(OsStr::new("in")) {
            inputs.push(entry.path());
        }
    }
    inputs.sort();
    Ok(inputs)
}

fn normalize(bytes: &[u8], options: &Test) -> Vec<u8> {
    let mut normalized: Vec<u8> = if options.ignore_line_ending {
        bytes
            .split_inclusive(|b| *b == b'\n')
            .flat_map(|line| match line.strip_suffix(b"\r\n") {
                Some(prefix) => [prefix, b"\n"].concat(),
                None => line.to_vec(),
            })
            .collect()
    } else {
        bytes.to_vec()
    };
    if options.strip_trailing_newline {
        while normalized
            .last()
            .is_some_and(|b| matches!(b, b'\r' | b'\n'))
        {
            normalized.pop();
        }
    }
    if !options.strip {
        return normalized;
    }
    let mut stripped = Vec::new();
    for line in normalized.split_inclusive(|b| *b == b'\n') {
        let (content, ending): (&[u8], &[u8]) = if let Some(content) = line.strip_suffix(b"\r\n") {
            (content, b"\r\n")
        } else if let Some(content) = line.strip_suffix(b"\n") {
            (content, b"\n")
        } else {
            (line, b"")
        };
        let end = content
            .iter()
            .rposition(|b| !matches!(b, b' ' | b'\t'))
            .map_or(0, |i| i + 1);
        stripped.extend_from_slice(&content[..end]);
        stripped.extend_from_slice(ending);
    }
    while stripped.last().is_some_and(u8::is_ascii_whitespace) {
        stripped.pop();
    }
    stripped
}

fn matches(expected: &[u8], actual: &[u8], options: &Test) -> bool {
    let expected = normalize(expected, options);
    let actual = normalize(actual, options);
    let Some(epsilon) = options.float_error else {
        return expected == actual;
    };
    let expected: Vec<_> = expected
        .split(u8::is_ascii_whitespace)
        .filter(|t| !t.is_empty())
        .collect();
    let actual: Vec<_> = actual
        .split(u8::is_ascii_whitespace)
        .filter(|t| !t.is_empty())
        .collect();
    expected.len() == actual.len()
        && expected.iter().zip(actual).all(|(e, a)| {
            if *e == a {
                return true;
            }
            let number = |s: &[u8]| {
                std::str::from_utf8(s)
                    .ok()?
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite())
            };
            let (Some(e), Some(a)) = (number(e), number(a)) else {
                return false;
            };
            let delta = (e - a).abs();
            let absolute = delta <= epsilon;
            let relative = if e == 0.0 {
                a == 0.0
            } else {
                (a / e - 1.0).abs() <= epsilon
            };
            match options.float_error_type {
                FloatErrorType::Both => absolute || relative,
                FloatErrorType::Absolute => absolute,
                FloatErrorType::Relative => relative,
            }
        })
}

fn print_io(label: &str, path: &Path, style: Style) -> Result<()> {
    let contents = fs::read(path)?;
    println!("{}", style.apply_to(format!("{label}:")));
    if contents.is_empty() {
        println!("{}", Style::new().dim().apply_to("(empty)"));
    } else {
        let contents = String::from_utf8_lossy(&contents);
        print!("{contents}");
        if !console::strip_ansi_codes(&contents).ends_with('\n') {
            print!(" {}", Style::new().dim().apply_to("(no eol)"));
        }
        println!();
    }
    Ok(())
}

fn run_jobs<T: Send>(
    tasks: Vec<T>,
    jobs: usize,
    fast_fail: bool,
    interrupted: &AtomicBool,
    run: impl Fn(T) -> Result<bool> + Sync,
) -> Result<(usize, usize)> {
    let workers = jobs.min(tasks.len());
    let tasks = Mutex::new(tasks.into_iter());
    let stopped = AtomicBool::new(false);
    thread::scope(|scope| {
        let workers: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| -> Result<(usize, usize)> {
                    let mut accepted = 0;
                    let mut total = 0;
                    loop {
                        let task = {
                            let mut tasks = tasks.lock().expect("task queue poisoned");
                            if stopped.load(Ordering::Relaxed)
                                || interrupted.load(Ordering::Relaxed)
                            {
                                break;
                            }
                            let Some(task) = tasks.next() else { break };
                            task
                        };
                        let passed =
                            run(task).inspect_err(|_| stopped.store(true, Ordering::Relaxed))?;
                        total += 1;
                        accepted += usize::from(passed);
                        if fast_fail && !passed {
                            stopped.store(true, Ordering::Relaxed);
                        }
                    }
                    Ok((accepted, total))
                })
            })
            .collect();
        let mut accepted = 0;
        let mut total = 0;
        for worker in workers {
            let (passed, finished) = worker
                .join()
                .map_err(|_| anyhow::anyhow!("Worker panicked"))??;
            accepted += passed;
            total += finished;
        }
        ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
        Ok((accepted, total))
    })
}

fn test_case(
    input: Option<PathBuf>,
    program: &Program,
    judge: Option<&Judge>,
    options: &Test,
    limits: Limits,
    interrupted: &AtomicBool,
) -> Result<bool> {
    let mut expected = input.as_ref().map(|p| p.with_extension("out"));
    let name = match &input {
        Some(p) => p
            .file_stem()
            .context("Invalid case name")?
            .to_string_lossy()
            .into_owned(),
        None => "interactive".into(),
    };
    let _empty_expected = if let Some(path) = &mut expected
        && !path.try_exists()?
    {
        if judge.is_some() {
            let file = tempfile::NamedTempFile::new()?;
            *path = file.path().to_owned();
            Some(file)
        } else {
            tracing::warn!(
                "Missing expected output for {name}; only the exit code will be checked"
            );
            None
        }
    } else {
        None
    };
    tracing::info!("Running test case {name}...");
    let actual = tempfile::NamedTempFile::new()?;
    let result = if options.interactive {
        let judge = judge
            .as_ref()
            .context("Interactive tests require --judge")?
            .command(input.as_deref(), expected.as_deref(), None)?;
        let transcript = if options.show_io == ShowIo::Never {
            None
        } else {
            Some(actual.reopen()?)
        };
        interactive(program, &judge, limits, interrupted, transcript)?
    } else {
        let mut result = execute(
            program,
            File::open(input.as_ref().expect("regular case"))?.into(),
            actual.reopen()?.into(),
            limits,
            interrupted,
        )?;
        if result.verdict == Verdict::Ac {
            let correct = if let Some(judge) = &judge {
                let command =
                    judge.command(input.as_deref(), expected.as_deref(), Some(actual.path()))?;
                execute(
                    &command,
                    Stdio::null(),
                    Stdio::inherit(),
                    limits,
                    interrupted,
                )?
                .verdict
                    == Verdict::Ac
            } else {
                let expected = expected.as_ref().expect("regular case");
                !expected.try_exists()?
                    || matches(&fs::read(expected)?, &fs::read(actual.path())?, options)
            };
            if !correct {
                result.verdict = Verdict::Wa;
            }
        }
        result
    };
    let _output = io::stdout().lock();
    println!(
        "{}: {} ({} ms, {} KiB)",
        name,
        crate::results::color_status(&result.verdict.to_string()),
        result.elapsed.as_millis(),
        result.memory / 1024
    );
    if match options.show_io {
        ShowIo::Always => true,
        ShowIo::Failure => result.verdict != Verdict::Ac,
        ShowIo::Never => false,
    } {
        let style = Style::new().bold();
        if let Some(input) = &input {
            print_io("Input", input, style.clone())?;
        }
        if let Some(expected) = &expected
            && expected.try_exists()?
        {
            print_io("Expected output", expected, style.clone().green())?;
        }
        let (label, style) = if options.interactive {
            ("Interaction", style)
        } else {
            ("Actual output", style.yellow())
        };
        print_io(label, actual.path(), style)?;
    }
    Ok(result.verdict == Verdict::Ac)
}

pub fn test(config: &Config, options: &Test, interrupted: &AtomicBool) -> Result<bool> {
    let program = Program::prepare(config, &options.program, interrupted)?;
    let directory = match &options.test_dir {
        Some(dir) => std::path::absolute(expand_path(dir)?)?,
        None => program.cwd.join("test"),
    };
    let cases = inputs(&directory)?;
    ensure!(
        options.interactive || !cases.is_empty(),
        "No .in test cases in {}",
        directory.display()
    );
    let judge = options
        .judge
        .as_ref()
        .map(|command| Judge::prepare(config, command, interrupted))
        .transpose()?;
    let limits = Limits {
        time: options.time_limit.map(|n| Duration::from_millis(n.get())),
        memory: options
            .memory_limit
            .map(|n| {
                n.get()
                    .checked_mul(1024 * 1024)
                    .context("Memory limit is too large")
            })
            .transpose()?,
    };
    let cases: Vec<_> = if cases.is_empty() {
        vec![None]
    } else {
        cases.into_iter().map(Some).collect()
    };
    tracing::info!(
        "Running {} test case(s) from {}...",
        cases.len(),
        directory.display()
    );
    let (accepted, total) = run_jobs(
        cases,
        options.jobs.get(),
        options.fast_fail,
        interrupted,
        |input| {
            test_case(
                input,
                &program,
                judge.as_ref(),
                options,
                limits,
                interrupted,
            )
        },
    )?;
    if total == 0 {
        tracing::warn!("No test cases were run");
    } else if accepted == total {
        tracing::info!("All {total} test case(s) passed");
    } else {
        tracing::warn!("{} of {} test case(s) passed", accepted, total);
    }
    Ok(accepted == total)
}

pub fn generate(config: &Config, options: &Generate, interrupted: &AtomicBool) -> Result<()> {
    let program = Program::prepare(config, &options.program, interrupted)?;
    let directory = std::path::absolute(expand_path(&options.dir)?)?;
    fs::create_dir_all(&directory)?;
    let mut tasks = Vec::new();
    if options.answer {
        tracing::info!("Generating missing answers in {}...", directory.display());
        for input in inputs(&directory)? {
            ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
            let output = input.with_extension("out");
            if !output.try_exists()? {
                tasks.push((Some(input), output));
            }
        }
    } else {
        tracing::info!(
            "Generating {} test case(s) in {}...",
            options.count,
            directory.display()
        );
        let mut index = 1usize;
        for _ in 0..options.count.get() {
            let output = loop {
                ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
                let path = directory.join(format!("random-{index:04}.in"));
                index += 1;
                if !path.try_exists()? && !path.with_extension("out").try_exists()? {
                    break path;
                }
            };
            tasks.push((None, output));
        }
    }
    let (_, count) = run_jobs(
        tasks,
        options.jobs.get(),
        false,
        interrupted,
        |(input, output)| {
            let staging = tempfile::Builder::new()
                .prefix(".cpg-")
                .tempfile_in(&directory)?;
            let stdin = match input.as_deref() {
                Some(p) => File::open(p)?.into(),
                None => Stdio::null(),
            };
            let result = execute(
                &program,
                stdin,
                staging.reopen()?.into(),
                Limits::default(),
                interrupted,
            )?;
            ensure!(
                result.verdict == Verdict::Ac,
                "Generator/reference solution failed ({})",
                result.verdict
            );
            staging
                .persist_noclobber(&output)
                .with_context(|| format!("Cannot save {}", output.display()))?;
            println!("{}", output.display());
            Ok(true)
        },
    )?;
    tracing::info!("Generated {count} file(s) in {}", directory.display());
    Ok(())
}

pub fn install_signal_handler() -> Result<Arc<AtomicBool>> {
    let interrupted = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, interrupted.clone())?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, interrupted.clone())?;
    Ok(interrupted)
}
