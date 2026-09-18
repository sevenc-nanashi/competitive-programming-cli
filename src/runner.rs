use crate::{
    cli::{FloatErrorType, Generate, Highlight, Panes, ProgramArgs, ShowIo, Test},
    config::{Clipboard, Config, Language, expand_path},
};
use anyhow::{Context, Result, ensure};
use console::{Alignment, Style, measure_text_width, pad_str};
use process_wrap::std::{ChildWrapper, CommandWrap};
use std::{
    ffi::{OsStr, OsString},
    fmt::Write as _,
    fs::{self, File},
    io::{self, IsTerminal, Read, Write},
    ops::Range,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
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
    prepared_source: Option<Arc<tempfile::TempPath>>,
    quiet: bool,
}

impl Program {
    fn shell(command: String, cwd: PathBuf) -> Self {
        Self {
            invocation: Invocation::Shell(command),
            cwd,
            prepared_source: None,
            quiet: false,
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
            let binary = file.with_extension(std::env::consts::EXE_EXTENSION);
            ensure!(
                compile.is_none() || binary != file,
                "Compiled output would overwrite the source file"
            );
            let prepared_source = prepare_source(language, &file, false, interrupted)?;
            let input: &Path = match &prepared_source {
                Some(source) => source,
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
                    result.verdict.on_compile()
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
                quiet: false,
            })
        }
    }
}

fn quote(value: &OsStr) -> Result<String> {
    let value = value.to_str().context(
        "Shell placeholders require UTF-8 paths; use a direct command for non-UTF-8 paths",
    )?;
    #[cfg(unix)]
    {
        Ok(shlex::try_quote(value)?.into_owned())
    }
    #[cfg(windows)]
    {
        let value = dunce::simplified(Path::new(value))
            .to_str()
            .context("Shell paths must be UTF-8")?;
        // Expand literal percent signs once, without interpreting path components as variables.
        Ok(format!(
            "\"{}\"",
            value.replace('%', "%CPG_LITERAL_PERCENT%")
        ))
    }
}

pub fn prepare_source(
    language: &Language,
    input: &Path,
    for_submission: bool,
    interrupted: &AtomicBool,
) -> Result<Option<Arc<tempfile::TempPath>>> {
    let mut output: Option<Arc<tempfile::TempPath>> = None;
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
            let input: &Path = match &output {
                Some(source) => source,
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
) -> Result<tempfile::TempPath> {
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
        .tempfile_in(&cwd)?
        // Close the handle so Windows commands can open the output for exclusive writing.
        .into_temp_path();
    let uses_processed = command.contains("{processed}");
    let processed = quote(output.as_os_str())?;
    let command = command
        .split("{input}")
        .map(|part| part.replace("{processed}", &processed))
        .collect::<Vec<_>>()
        .join(&quote(input.as_os_str())?);
    let program = Program::shell(command, cwd);
    tracing::info!("Running {stage} for {}", input.display());
    let result = execute(
        &program,
        File::open(input)?.into(),
        if uses_processed {
            io::stderr().into()
        } else {
            File::create(&output)?.into()
        },
        Limits::default(),
        interrupted,
    )?;
    ensure!(
        result.verdict == Verdict::Ac,
        "{stage} failed ({})",
        result.verdict.on_compile()
    );
    let source =
        fs::read_to_string(&output).with_context(|| format!("{stage} output must be UTF-8"))?;
    ensure!(
        !source.trim().is_empty(),
        "{stage} produced empty output; configure it to write source code to {}",
        if uses_processed {
            "{processed}"
        } else {
            "stdout"
        }
    );
    Ok(output)
}

struct ManagedChild {
    child: Box<dyn ChildWrapper>,
    status: Option<ExitStatus>,
    kill_group: bool,
}

impl ManagedChild {
    fn spawn(program: &Program, stdin: Stdio, stdout: Stdio) -> Result<Self> {
        let mut command = match &program.invocation {
            Invocation::Shell(script) => {
                #[cfg(unix)]
                {
                    let mut command = Command::new("sh");
                    command.args(["-c", script]);
                    command
                }
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    let mut command = Command::new("cmd.exe");
                    command
                        .args(["/D", "/E:ON", "/V:OFF", "/S", "/C"])
                        .raw_arg(format!("\"{script}\""))
                        .env("CPG_LITERAL_PERCENT", "%");
                    command
                }
            }
            Invocation::Direct(argv) => {
                let (executable, args) = argv.split_first().context("Empty command")?;
                let mut command = Command::new(executable);
                command.args(args);
                command
            }
        };
        let executable = command.get_program().to_owned();
        let cwd = &program.cwd;
        #[cfg(windows)]
        let cwd = dunce::simplified(cwd);
        command
            .current_dir(cwd)
            .stdin(stdin)
            .stdout(stdout)
            .stderr(if program.quiet {
                Stdio::null()
            } else {
                Stdio::inherit()
            });
        let mut command = CommandWrap::from(command);
        #[cfg(unix)]
        command.wrap(process_wrap::std::ProcessGroup::leader());
        #[cfg(windows)]
        command.wrap(process_wrap::std::JobObject);
        let child = command
            .spawn()
            .with_context(|| format!("Cannot start {}", executable.to_string_lossy()))?;
        Ok(Self {
            child,
            status: None,
            kill_group: true,
        })
    }

    fn poll(&mut self) -> Result<Option<ExitStatus>> {
        if self.status.is_none() {
            self.status = self.child.inner_mut().try_wait()?;
        }
        Ok(self.status)
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        // The shell and all children inheriting its process group are owned by this run.
        if self.kill_group {
            let _ = self.child.start_kill();
            let _ = self.child.wait();
        } else {
            let _ = self.child.inner_mut().wait();
        }
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
    Ce,
}
impl Verdict {
    fn on_compile(&self) -> Self {
        match self {
            Self::Ac => Self::Ac,
            _ => Self::Ce,
        }
    }
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Ac => "AC",
            Self::Wa => "WA",
            Self::Re => "RE",
            Self::Tle => "TLE",
            Self::Mle => "MLE",
            Self::Ce => "CE",
        })
    }
}

struct RunResult {
    verdict: Verdict,
    elapsed: Duration,
    memory: u64,
}

fn monitor(
    children: &mut [&mut ManagedChild],
    limits: Limits,
    interrupted: &AtomicBool,
) -> Result<RunResult> {
    let started = Instant::now();
    let mut peak = 0;
    let mut memory = crate::platform::MemoryMonitor::default();
    let verdict = loop {
        ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
        peak = peak.max(memory.usage(children[0].child.id())?);
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

#[cfg(feature = "mock")]
pub fn judge_samples(
    source: &Path,
    compile: Option<&str>,
    run: &str,
    samples: &[crate::model::Sample],
) -> Result<(String, Duration)> {
    ensure!(!samples.is_empty(), "No sample test cases to judge");
    let directory = source.parent().context("Source file has no parent")?;
    let binary = source.with_extension(std::env::consts::EXE_EXTENSION);
    let program = |command: &str| -> Result<Program> {
        let command = command
            .replace("{input}", &quote(source.as_os_str())?)
            .replace("{binary}", &quote(binary.as_os_str())?);
        let mut program = Program::shell(command, directory.to_owned());
        program.quiet = true;
        Ok(program)
    };
    let interrupted = AtomicBool::new(false);
    if let Some(compile) = compile {
        let result = execute(
            &program(compile)?,
            Stdio::null(),
            Stdio::null(),
            Limits {
                time: Some(Duration::from_secs(30)),
                memory: None,
            },
            &interrupted,
        )?;
        if result.verdict != Verdict::Ac {
            return Ok(("CE".into(), Duration::ZERO));
        }
    }
    let program = program(run)?;
    let input = directory.join("sample.in");
    let actual = directory.join("sample.out");
    let mut elapsed = Duration::ZERO;
    for sample in samples {
        fs::write(&input, &sample.input)?;
        let result = execute(
            &program,
            File::open(&input)?.into(),
            File::create(&actual)?.into(),
            Limits {
                time: Some(Duration::from_secs(2)),
                memory: None,
            },
            &interrupted,
        )?;
        elapsed = elapsed.max(result.elapsed);
        let verdict =
            if result.verdict == Verdict::Ac && fs::read(&actual)? != sample.output.as_bytes() {
                Verdict::Wa
            } else {
                result.verdict
            };
        if verdict != Verdict::Ac {
            return Ok((verdict.to_string(), elapsed));
        }
    }
    Ok((Verdict::Ac.to_string(), elapsed))
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

pub fn copy_to_clipboard(
    clipboard: &Clipboard,
    content: &str,
    interrupted: &AtomicBool,
) -> Result<()> {
    ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
    match clipboard {
        Clipboard::Osc52 { .. } => crossterm::execute!(
            io::stderr(),
            crossterm::clipboard::CopyToClipboard::to_clipboard_from(content)
        )
        .context("Cannot write OSC 52 to the terminal"),
        // ponytail: persistence after exit relies on a clipboard manager; use command for wl-copy/xclip.
        Clipboard::Arboard { .. } => arboard::Clipboard::new()
            .context("Cannot open the system clipboard")?
            .set_text(content)
            .context("Cannot copy text to the clipboard"),
        Clipboard::Command { command, .. } => {
            ensure!(
                !command.trim().is_empty(),
                "Clipboard command must not be empty"
            );
            let program = Program::shell(command.clone(), std::env::current_dir()?);
            let mut child = ManagedChild::spawn(&program, Stdio::piped(), io::stderr().into())?;
            let mut stdin = child
                .child
                .stdin()
                .take()
                .context("Missing clipboard command stdin")?;
            thread::scope(|scope| {
                let writer = scope.spawn(move || stdin.write_all(content.as_bytes()));
                let result = monitor(&mut [&mut child], Limits::default(), interrupted);
                if matches!(&result, Ok(result) if result.verdict == Verdict::Ac) {
                    writer
                        .join()
                        .expect("Clipboard writer panicked")
                        .context("Cannot write to clipboard command")?;
                    // Clipboard tools such as wl-copy keep serving data in a background process.
                    child.kill_group = false;
                }
                drop(child);
                let result = result?;
                ensure!(
                    result.verdict == Verdict::Ac,
                    "Clipboard command failed ({})",
                    result.verdict
                );
                Ok(())
            })
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct TranscriptLine {
    solution: bool,
    turn: usize,
    text: String,
    no_eol: bool,
}

struct Transcript {
    file: File,
    numbered: bool,
    panes: bool,
    solution: Option<bool>,
    turn: usize,
    pending: Vec<u8>,
}

impl Transcript {
    fn flush_line(&mut self, no_eol: bool) -> io::Result<()> {
        let solution = self.solution.expect("transcript speaker");
        if self.panes {
            serde_json::to_writer(
                &mut self.file,
                &TranscriptLine {
                    solution,
                    turn: self.turn,
                    text: String::from_utf8_lossy(&self.pending).into_owned(),
                    no_eol,
                },
            )?;
            writeln!(self.file)?;
            self.pending.clear();
            return Ok(());
        }
        let (prefix, style) = if solution {
            (">", Style::new().yellow())
        } else {
            ("<", Style::new().green())
        };
        write!(
            self.file,
            "{} {} {}",
            self.turn,
            style.apply_to(prefix),
            String::from_utf8_lossy(&self.pending)
        )?;
        if no_eol {
            writeln!(self.file, " {}", Style::new().dim().apply_to("(no eol)"))?;
        }
        self.pending.clear();
        Ok(())
    }

    fn record(&mut self, solution: bool, bytes: &[u8]) -> io::Result<()> {
        if !self.numbered && !self.panes {
            let (prefix, style) = if solution {
                ("> ", Style::new().yellow())
            } else {
                ("< ", Style::new().green())
            };
            return write!(
                self.file,
                "{}{}",
                style.apply_to(prefix),
                String::from_utf8_lossy(bytes)
            );
        }
        if self.solution != Some(solution) {
            if !self.pending.is_empty() {
                self.flush_line(true)?;
            }
            if solution {
                self.turn += 1;
            }
            self.solution = Some(solution);
        }
        for part in bytes.split_inclusive(|byte| *byte == b'\n') {
            self.pending.extend_from_slice(part);
            if part.ends_with(b"\n") {
                self.flush_line(false)?;
            }
        }
        Ok(())
    }
}

fn relay(
    mut input: impl Read,
    mut output: impl Write,
    solution: bool,
    transcript: Option<Arc<Mutex<Transcript>>>,
) -> io::Result<()> {
    let mut buffer = [0; 4096];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            return Ok(());
        }
        if let Some(transcript) = &transcript {
            transcript
                .lock()
                .expect("transcript lock poisoned")
                .record(solution, &buffer[..n])?;
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
    query_numbers: bool,
    panes: bool,
) -> Result<RunResult> {
    let mut solution = ManagedChild::spawn(program, Stdio::piped(), Stdio::piped())?;
    let mut judge = ManagedChild::spawn(judge, Stdio::piped(), Stdio::piped())?;
    let solution_out = solution.child.stdout().take().expect("piped stdout");
    let judge_in = judge.child.stdin().take().expect("piped stdin");
    let judge_out = judge.child.stdout().take().expect("piped stdout");
    let solution_in = solution.child.stdin().take().expect("piped stdin");
    let transcript = transcript.map(|file| {
        Arc::new(Mutex::new(Transcript {
            file,
            numbered: query_numbers,
            panes,
            solution: None,
            turn: 0,
            pending: Vec::new(),
        }))
    });
    let forward_transcript = transcript.clone();
    let backward_transcript = transcript.clone();
    let forward = thread::spawn(move || relay(solution_out, judge_in, true, forward_transcript));
    let backward = thread::spawn(move || relay(judge_out, solution_in, false, backward_transcript));
    let result = monitor(&mut [&mut solution, &mut judge], limits, interrupted);
    drop(solution);
    drop(judge);
    let forwarded = forward
        .join()
        .map_err(|_| anyhow::anyhow!("Solution relay panicked"))?;
    let backwarded = backward
        .join()
        .map_err(|_| anyhow::anyhow!("Judge relay panicked"))?;
    if let Some(transcript) = &transcript {
        let mut transcript = transcript.lock().expect("transcript lock poisoned");
        if !transcript.pending.is_empty() {
            transcript.flush_line(true)?;
        }
    }
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
                quiet: false,
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

struct OutputLine {
    text: String,
    marker_start: usize,
    highlights: Vec<Range<usize>>,
    highlight_style: Style,
}

impl OutputLine {
    fn new(text: &str, marker: &str) -> Self {
        let mut plain = String::new();
        for ch in console::strip_ansi_codes(text).chars() {
            if ch == '\t' {
                plain.push_str(&" ".repeat(8 - measure_text_width(&plain) % 8));
            } else if ch.is_control() {
                plain.extend(ch.escape_default());
            } else {
                plain.push(ch);
            }
        }
        let marker_start = plain.len();
        plain.push_str(marker);
        Self {
            text: plain,
            marker_start,
            highlights: Vec::new(),
            highlight_style: Style::new(),
        }
    }

    fn highlight(&mut self, other: Option<&Self>, mode: Highlight, style: Style) {
        self.highlight_style = style;
        let Some(other) = other else {
            self.highlights.push(0..self.text.len());
            return;
        };
        if self.text == other.text {
            return;
        }
        if mode == Highlight::Line {
            self.highlights.push(0..self.text.len());
            return;
        }
        let mut offset = 0;
        let mut other_words = other.text.split_whitespace();
        for part in self.text.split_inclusive(char::is_whitespace) {
            let word = part.trim_end_matches(char::is_whitespace);
            if !word.is_empty() && other_words.next() != Some(word) {
                self.highlights.push(offset..offset + word.len());
            }
            offset += part.len();
        }
    }

    fn wrap(&self, width: usize) -> Vec<String> {
        assert!(width >= 2, "a pane must fit a wide character");
        let mut lines = vec![String::new()];
        let mut columns = 0;
        let mut highlights = self.highlights.iter().peekable();
        for (offset, ch) in self.text.char_indices() {
            let text = ch.to_string();
            let size = measure_text_width(&text);
            if columns + size > width {
                lines.push(String::new());
                columns = 0;
            }
            let line = lines.last_mut().expect("initial line");
            while highlights.peek().is_some_and(|range| range.end <= offset) {
                highlights.next();
            }
            if highlights
                .peek()
                .is_some_and(|range| range.contains(&offset))
            {
                write!(line, "{}", self.highlight_style.apply_to(text)).expect("write to string");
            } else if offset >= self.marker_start {
                write!(line, "{}", Style::new().dim().apply_to(text)).expect("write to string");
            } else {
                line.push(ch);
            }
            columns += size;
        }
        lines
    }
}

fn interaction_panes(
    lines: &[TranscriptLine],
    numbered: bool,
    width: Option<usize>,
) -> Result<String> {
    let digits = lines.last().map_or(1, |line| line.turn.to_string().len());
    let gutter = if numbered { digits + 6 } else { 3 };
    let content: Vec<_> = lines
        .iter()
        .map(|line| {
            let text = if line.no_eol {
                line.text.as_str()
            } else {
                let text = line
                    .text
                    .strip_suffix('\n')
                    .expect("complete transcript line");
                match text.strip_suffix('\r') {
                    Some(text) => text,
                    None => text,
                }
            };
            OutputLine::new(text, if line.no_eol { " (no eol)" } else { "" })
        })
        .collect();
    let widths = if let Some(width) = width {
        ensure!(
            width >= gutter + 4,
            "Terminal is too narrow for the selected panes"
        );
        let available = width - gutter;
        [available.div_ceil(2), available / 2]
    } else {
        let mut widths = [6, 9];
        for (line, content) in lines.iter().zip(&content) {
            let column = usize::from(line.solution);
            widths[column] = widths[column].max(measure_text_width(&content.text));
        }
        widths
    };
    let headers = [
        OutputLine::new("Judge:", "").wrap(widths[0]),
        OutputLine::new("Solution:", "").wrap(widths[1]),
    ];
    let mut output = String::new();
    for row in 0..headers[0].len().max(headers[1].len()) {
        let left = headers[0].get(row).map_or("", String::as_str);
        let right = headers[1].get(row).map_or("", String::as_str);
        writeln!(
            output,
            "{}{}{}",
            Style::new()
                .green()
                .bold()
                .apply_to(pad_str(left, widths[0], Alignment::Left, None)),
            " ".repeat(gutter),
            Style::new().yellow().bold().apply_to(right)
        )
        .expect("write to string");
    }
    for (line, content) in lines.iter().zip(content) {
        let column = usize::from(line.solution);
        for (row, text) in content.wrap(widths[column]).iter().enumerate() {
            let (left, right) = if line.solution {
                ("", text.as_str())
            } else {
                (text.as_str(), "")
            };
            let style = if line.solution {
                Style::new().yellow()
            } else {
                Style::new().green()
            };
            let separator = if numbered {
                let number = if row == 0 {
                    format!("{:>digits$}", line.turn)
                } else {
                    " ".repeat(digits)
                };
                if line.solution {
                    format!(" : {} ", style.apply_to(format!("{number} |")))
                } else {
                    format!(" {} : ", style.apply_to(format!("| {number}")))
                }
            } else if line.solution {
                format!(" {} ", style.apply_to("<"))
            } else {
                format!(" {} ", style.apply_to(">"))
            };
            writeln!(
                output,
                "{}{}{}",
                pad_str(left, widths[0], Alignment::Left, None),
                separator,
                right
            )
            .expect("write to string");
        }
    }
    if lines.is_empty() {
        writeln!(output, "{}", Style::new().dim().apply_to("(empty)")).expect("write to string");
    }
    output.push('\n');
    Ok(output)
}

struct OutputBlock {
    label: &'static str,
    style: Style,
    lines: Vec<OutputLine>,
    count: usize,
}

impl OutputBlock {
    fn read(label: &'static str, path: Option<&Path>, style: Style) -> Result<Self> {
        let contents = path.map(fs::read).transpose()?;
        let mut lines = Vec::new();
        if let Some(contents) = &contents {
            let contents = String::from_utf8_lossy(contents);
            let contents = console::strip_ansi_codes(&contents);
            for line in contents.split_inclusive('\n') {
                match line.strip_suffix('\n') {
                    Some(line) => {
                        let line = match line.strip_suffix('\r') {
                            Some(line) => line,
                            None => line,
                        };
                        lines.push(OutputLine::new(line, ""));
                    }
                    None => lines.push(OutputLine::new(line, " (no eol)")),
                }
            }
        }
        let count = lines.len();
        if lines.is_empty() {
            lines.push(OutputLine::new(
                "",
                if contents.is_some() {
                    "(empty)"
                } else {
                    "(missing)"
                },
            ));
        }
        Ok(Self {
            label,
            style,
            lines,
            count,
        })
    }

    fn vertical(&self, numbered: bool, expected_count: usize, digits: usize) -> String {
        let mut output = format!("{}\n", self.style.apply_to(format!("{}:", self.label)));
        for (index, line) in self.lines.iter().enumerate() {
            if numbered {
                let number = if self.count == 0 {
                    None
                } else {
                    (index + expected_count + 1)
                        .checked_sub(self.count)
                        .filter(|n| *n > 0)
                };
                match number {
                    Some(number) => write!(output, "{number:>digits$} | "),
                    None => write!(output, "{} | ", " ".repeat(digits)),
                }
                .expect("write to string");
            }
            writeln!(output, "{}", line.wrap(usize::MAX)[0]).expect("write to string");
        }
        output.push('\n');
        output
    }
}

fn pane_output(
    blocks: &[&OutputBlock],
    numbered: bool,
    terminal_width: Option<usize>,
) -> Result<String> {
    let all = blocks.len() == 3;
    let expected = blocks[blocks.len() - 2];
    let actual = blocks[blocks.len() - 1];
    let count = expected.count.max(actual.count);
    let digits = count.to_string().len();
    let gutter = if numbered { digits + 3 } else { 0 };
    let widths = if let Some(width) = terminal_width {
        let spacing = gutter + (blocks.len() - 1) * 3;
        ensure!(
            width >= spacing + blocks.len() * 2,
            "Terminal is too narrow for the selected panes"
        );
        let available = width - spacing;
        (0..blocks.len())
            .map(|column| available / blocks.len() + usize::from(column < available % blocks.len()))
            .collect::<Vec<_>>()
    } else {
        blocks
            .iter()
            .map(|block| {
                block
                    .lines
                    .iter()
                    .map(|line| measure_text_width(&line.text))
                    .chain([block.label.len() + 1, 2])
                    .max()
                    .expect("label width")
            })
            .collect()
    };
    let mut output = String::new();
    let mut append =
        |cells: &[Vec<String>], starts: &[usize], number: Option<usize>, number_row: usize| {
            let height = cells
                .iter()
                .zip(starts)
                .map(|(cell, start)| cell.len() + start)
                .max()
                .expect("output columns");
            for row in 0..height {
                let number = number.filter(|_| row == number_row);
                if numbered {
                    if let Some(number) = number {
                        write!(output, "{number:>digits$}").expect("write to string");
                    } else {
                        output.push_str(&" ".repeat(digits));
                    }
                }
                for (column, cell) in cells.iter().enumerate() {
                    let text = row
                        .checked_sub(starts[column])
                        .and_then(|row| cell.get(row));
                    if numbered || column > 0 {
                        output.push_str(if text.is_some() { " | " } else { " : " });
                    }
                    let text = match text {
                        Some(text) => text.as_str(),
                        None => "",
                    };
                    output.push_str(&pad_str(text, widths[column], Alignment::Left, None));
                }
                output.push('\n');
            }
        };
    let headers: Vec<_> = blocks
        .iter()
        .zip(&widths)
        .map(|(block, width)| {
            OutputLine::new(&format!("{}:", block.label), "")
                .wrap(*width)
                .into_iter()
                .map(|line| block.style.apply_to(line).to_string())
                .collect()
        })
        .collect();
    append(&headers, &vec![0; blocks.len()], None, 0);
    let expected_start = if all {
        blocks[0].lines.len().saturating_sub(expected.count)
    } else {
        0
    };
    let mut offsets = vec![expected_start; blocks.len()];
    if all {
        offsets[0] = expected.count.saturating_sub(blocks[0].lines.len());
    }
    let rows = blocks
        .iter()
        .zip(&offsets)
        .map(|(block, offset)| block.lines.len() + offset)
        .max()
        .expect("output columns");
    for row in 0..rows {
        let cells: Vec<_> = blocks
            .iter()
            .zip(&offsets)
            .zip(&widths)
            .map(|((block, offset), width)| {
                match row
                    .checked_sub(*offset)
                    .and_then(|row| block.lines.get(row))
                {
                    Some(line) => line.wrap(*width),
                    None => Vec::new(),
                }
            })
            .collect();
        let mut starts = vec![0; blocks.len()];
        if all {
            let input_height = cells[0].len();
            let expected_height = cells[1].len();
            starts[0] = expected_height.saturating_sub(input_height);
            starts[1] = input_height.saturating_sub(expected_height);
            starts[2] = starts[1];
        }
        let number = row
            .checked_sub(expected_start)
            .filter(|index| *index < count)
            .map(|index| index + 1);
        append(&cells, &starts, number, starts[blocks.len() - 2]);
    }
    Ok(output)
}

fn print_test_io(
    input: &Path,
    expected: Option<&Path>,
    actual: &Path,
    options: &Test,
) -> Result<()> {
    let style = Style::new().bold();
    let has_expected = expected.is_some();
    let input = OutputBlock::read("Input", Some(input), style.clone())?;
    let mut expected = OutputBlock::read("Expected output", expected, style.clone().green())?;
    let mut actual = OutputBlock::read("Actual output", Some(actual), style.yellow())?;
    if let Some(highlight) = options.highlight
        && has_expected
    {
        for (index, line) in expected.lines.iter_mut().enumerate() {
            line.highlight(actual.lines.get(index), highlight, Style::new().green());
        }
        for (index, line) in actual.lines.iter_mut().enumerate() {
            line.highlight(expected.lines.get(index), highlight, Style::new().red());
        }
    }
    let digits = expected.count.max(actual.count).to_string().len();
    if options.panes != Panes::All {
        print!(
            "{}",
            input.vertical(options.query_numbers, expected.count, digits)
        );
    }
    if options.panes == Panes::None {
        if has_expected {
            print!(
                "{}",
                expected.vertical(options.query_numbers, expected.count, digits)
            );
        }
        print!(
            "{}",
            actual.vertical(options.query_numbers, actual.count, digits)
        );
    } else {
        let width = if io::stdout().is_terminal() {
            Some(usize::from(
                console::Term::stdout()
                    .size_checked()
                    .context("Cannot determine terminal width")?
                    .1,
            ))
        } else {
            None
        };
        let blocks = if options.panes == Panes::All {
            vec![&input, &expected, &actual]
        } else {
            vec![&expected, &actual]
        };
        print!("{}", pane_output(&blocks, options.query_numbers, width)?);
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
    let empty_expected = if let Some(path) = &mut expected
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
        interactive(
            program,
            &judge,
            limits,
            interrupted,
            transcript,
            options.query_numbers,
            options.panes == Panes::Outputs,
        )?
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
        if !options.interactive
            && (options.panes != Panes::None
                || options.query_numbers
                || options.highlight.is_some())
        {
            let expected = match &expected {
                Some(path) if path.try_exists()? && empty_expected.is_none() => {
                    Some(path.as_path())
                }
                _ => None,
            };
            print_test_io(
                input.as_deref().expect("regular case"),
                expected,
                actual.path(),
                options,
            )?;
            return Ok(result.verdict == Verdict::Ac);
        }
        let style = Style::new().bold();
        if let Some(input) = &input {
            print_io("Input", input, style.clone())?;
        }
        if let Some(expected) = &expected
            && expected.try_exists()?
        {
            print_io("Expected output", expected, style.clone().green())?;
        }
        if options.interactive && options.panes == Panes::Outputs {
            let lines = serde_json::Deserializer::from_reader(actual.reopen()?)
                .into_iter::<TranscriptLine>()
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let width = if io::stdout().is_terminal() {
                Some(usize::from(
                    console::Term::stdout()
                        .size_checked()
                        .context("Cannot determine terminal width")?
                        .1,
                ))
            } else {
                None
            };
            print!(
                "{}",
                interaction_panes(&lines, options.query_numbers, width)?
            );
            return Ok(result.verdict == Verdict::Ac);
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
    ensure!(
        !options.interactive || options.panes != Panes::All,
        "--interactive cannot be combined with --panes all"
    );
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
    #[cfg(unix)]
    {
        signal_hook::flag::register(signal_hook::consts::SIGINT, interrupted.clone())?;
        signal_hook::flag::register(signal_hook::consts::SIGTERM, interrupted.clone())?;
    }
    #[cfg(windows)]
    {
        let flag = interrupted.clone();
        ctrlc::set_handler(move || flag.store(true, Ordering::Relaxed))?;
    }
    Ok(interrupted)
}

#[cfg(test)]
mod display_tests {
    use super::*;

    #[test]
    fn highlights_compare_lines_or_words_without_changing_text() {
        for (mode, expected_ranges, actual_ranges) in [
            (Highlight::Line, vec!["same 猫 end"], vec!["same 犬 extra"]),
            (Highlight::Word, vec!["猫", "end"], vec!["犬", "extra"]),
        ] {
            let mut expected = OutputLine::new("same 猫 end", "");
            let mut actual = OutputLine::new("same 犬 extra", "");
            expected.highlight(Some(&actual), mode, Style::new().green());
            actual.highlight(Some(&expected), mode, Style::new().red());
            for (line, ranges) in [(&expected, expected_ranges), (&actual, actual_ranges)] {
                assert_eq!(
                    line.highlights
                        .iter()
                        .map(|range| &line.text[range.clone()])
                        .collect::<Vec<_>>(),
                    ranges
                );
                let wrapped = line.wrap(4);
                assert!(wrapped.iter().all(|line| measure_text_width(line) <= 4));
                assert_eq!(console::strip_ansi_codes(&wrapped.concat()), line.text);
            }
        }
        let mut line = OutputLine::new("same", "");
        line.highlight(
            Some(&OutputLine::new("same", "")),
            Highlight::Line,
            Style::new(),
        );
        assert!(line.highlights.is_empty());
        line.highlight(None, Highlight::Word, Style::new());
        assert_eq!(line.highlights.len(), 1);
        assert_eq!(line.highlights[0], 0..4);
        let mut line = OutputLine::new("same  words", "");
        line.highlight(
            Some(&OutputLine::new("same words", "")),
            Highlight::Word,
            Style::new(),
        );
        assert!(line.highlights.is_empty());
    }

    #[test]
    fn wrapped_panes_keep_corresponding_lines() {
        let block = |label, contents: &str| {
            let file = tempfile::NamedTempFile::new().unwrap();
            fs::write(file.path(), contents).unwrap();
            OutputBlock::read(label, Some(file.path()), Style::new()).unwrap()
        };
        let input = block("Input", "abcdefghijklmnopqrstuv\n");
        let expected = block("Expected output", "ok\n");
        let actual = block("Actual output", "abcdefghijklmnopqrstuvwxy\n");
        let blocks = [&input, &expected, &actual];
        let output = pane_output(&blocks, true, Some(40)).unwrap();
        assert!(output.lines().all(|line| measure_text_width(line) == 40));
        let rows: Vec<_> = output.lines().skip(2).map(str::trim_end).collect();
        assert_eq!(
            rows,
            [
                "  | abcdefghij :            :",
                "  | klmnopqrst :            :",
                "1 | uv         | ok         | abcdefghij",
                "  :            :            | klmnopqrst",
                "  :            :            | uvwxy",
            ]
        );
        assert!(pane_output(&blocks, true, Some(15)).is_err());
        assert!(pane_output(&blocks, true, Some(16)).is_ok());
        let blank = block("Input", "\n");
        let blank_output = pane_output(&[&blank, &blank, &blank], true, None).unwrap();
        assert!(blank_output.lines().nth(1).unwrap().starts_with("1 | "));
        assert_eq!(
            blank_output.lines().nth(1).unwrap().matches(" | ").count(),
            3
        );
        let colored = block("Expected output", "\x1b[31mok\n\x1b[0m");
        assert_eq!(colored.count, 1);
        assert_eq!(colored.lines[0].text, "ok");

        let line = OutputLine::new("界\tA\u{7}\x1b[31mB\x1b[0m", " (no eol)");
        for width in 2..12 {
            let wrapped = line.wrap(width);
            assert!(wrapped.iter().all(|line| measure_text_width(line) <= width));
            assert_eq!(
                console::strip_ansi_codes(&wrapped.concat()),
                "界      A\\u{7}B (no eol)"
            );
        }
    }

    #[test]
    fn transcript_numbers_exchanges_not_read_chunks() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut transcript = Transcript {
            file: file.reopen().unwrap(),
            numbered: true,
            panes: false,
            solution: None,
            turn: 0,
            pending: Vec::new(),
        };
        for (solution, bytes) in [
            (false, b"ini".as_slice()),
            (false, b"t\nmore\n"),
            (true, b"\xe3"),
            (true, b"\x81"),
            (true, b"\x82\nsecond\npartial"),
            (false, b"reply\n"),
            (true, b"done"),
        ] {
            transcript.record(solution, bytes).unwrap();
        }
        transcript.flush_line(true).unwrap();
        let output = fs::read_to_string(file.path()).unwrap();
        assert_eq!(
            console::strip_ansi_codes(&output),
            concat!(
                "0 < init\n0 < more\n1 > あ\n1 > second\n1 > partial (no eol)\n",
                "1 < reply\n2 > done (no eol)\n",
            )
        );
    }

    #[test]
    fn interactive_panes_preserve_speakers_and_wrapping() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut transcript = Transcript {
            file: file.reopen().unwrap(),
            numbered: false,
            panes: true,
            solution: None,
            turn: 0,
            pending: Vec::new(),
        };
        for (solution, bytes) in [
            (false, b"init\r\n\n".as_slice()),
            (true, b"\xe3"),
            (true, b"\x81\x82\n"),
            (false, b"reply"),
            (true, b"done"),
        ] {
            transcript.record(solution, bytes).unwrap();
        }
        transcript.flush_line(true).unwrap();
        let lines = serde_json::Deserializer::from_reader(file.reopen().unwrap())
            .into_iter::<TranscriptLine>()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        let plain = interaction_panes(&lines, false, None).unwrap();
        let plain = console::strip_ansi_codes(&plain);
        assert!(
            plain.contains("init           > \n               > \n               < あ\n"),
            "{plain}"
        );
        let numbered = interaction_panes(&lines, true, None).unwrap();
        let numbered = console::strip_ansi_codes(&numbered);
        assert!(
            numbered.contains("reply (no eol) | 1 : \n               : 2 | done (no eol)"),
            "{numbered}"
        );
        for width in 11..30 {
            let rendered = interaction_panes(&lines, true, Some(width)).unwrap();
            assert!(
                rendered
                    .lines()
                    .all(|line| measure_text_width(line) <= width)
            );
        }
        assert!(interaction_panes(&lines, true, Some(10)).is_err());
        assert!(
            interaction_panes(&[], false, None)
                .unwrap()
                .contains("(empty)")
        );
    }
}
