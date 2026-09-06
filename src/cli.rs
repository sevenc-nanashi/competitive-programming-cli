use crate::model::ServiceId;
use std::{
    ffi::OsString,
    num::{NonZeroU64, NonZeroUsize},
    path::PathBuf,
};
use url::Url;

/// Download, test, and submit competitive programming solutions.
#[derive(Debug, usage::Cli)]
#[usage(bin = "cpg", version)]
pub struct Cli {
    /// Disable colored output.
    #[usage(long, global)]
    pub no_color: bool,
    #[usage(subcommand)]
    pub command: Commands,
}

#[derive(Debug, usage::Subcommands)]
pub enum Commands {
    /// Initialize configuration and template directories interactively.
    Init(Init),
    /// Show the workspace root and configuration, cookies, and template directories.
    Config(Config),
    /// Import a Netscape cookie file and verify the session.
    Login(Login),
    /// Download one problem and its samples.
    #[usage(alias = "d")]
    Download(Download),
    /// Prepare all problems in a contest.
    #[usage(alias = "p")]
    Prepare(Download),
    /// Open the current problem or contest in your browser.
    #[usage(alias = "o")]
    Open(Open),
    /// Compile and test a solution.
    #[usage(alias = "t")]
    Test(Test),
    /// Generate test inputs or reference answers.
    #[usage(alias = "g")]
    Generate(Generate),
    /// Submit a source file.
    #[usage(alias = "s")]
    Submit(Submit),
    /// Show your submissions for the current problem or contest.
    #[usage(alias = "r")]
    Results(Results),
    /// List downloaded workspaces, contests, or problems.
    List(List),
}

#[derive(Debug, usage::Args)]
pub struct Init {
    /// Import [templates] from an oj-prepare configuration file.
    #[usage(long)]
    pub from_oj: Option<PathBuf>,
}

#[derive(Debug, usage::Args)]
pub struct Config {
    #[usage(arg_group)]
    pub field: Option<ConfigField>,
}

#[derive(Debug, Clone, Copy, usage::ArgGroup)]
#[usage(name = "field")]
pub enum ConfigField {
    /// Print only the absolute workspace root.
    Root,
    /// Print only the absolute configuration directory.
    ConfigDir,
    /// Print only the absolute cookies directory.
    CookiesDir,
    /// Print only the absolute workspace template directory.
    WorkspaceTemplateDir,
    /// Print only the absolute problem template directory.
    ProblemTemplateDir,
    /// Print only the absolute contest template directory.
    ContestTemplateDir,
    /// Print only the absolute single problem template directory.
    SingleProblemTemplateDir,
}

#[derive(Debug, usage::Args)]
pub struct Login {
    /// Online judge whose session should be imported and verified.
    #[usage(value_enum)]
    pub service: ServiceId,
    /// Netscape-format cookie file to import.
    #[usage(long)]
    pub cookie_file: PathBuf,
}

#[derive(Debug, usage::Args)]
pub struct Download {
    /// Problem or contest URL to download.
    pub url: Url,
}

#[derive(Debug, usage::Args)]
pub struct Open {
    /// Print the URL without opening a browser.
    #[usage(long, short = 'n')]
    pub url_only: bool,
}

#[derive(Debug, usage::Args)]
pub struct ProgramArgs {
    /// Source or executable file to compile/run using language configuration.
    ///
    /// Executable files without a matching extension use the executable language.
    #[usage(conflicts("command"), required_unless("command"))]
    pub file: Option<PathBuf>,
    /// Command and arguments after --, passed through unchanged.
    #[usage(double_dash = "required", conflicts("file"), required_unless("file"))]
    pub command: Vec<OsString>,
    /// Language compilation/run profile.
    #[usage(long)]
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, usage::ValueEnum)]
pub enum FloatErrorType {
    /// Accept when either absolute or relative error is within the tolerance.
    #[default]
    Both,
    /// Compare the absolute difference between expected and actual values.
    Absolute,
    /// Compare relative to the expected value; an expected zero requires zero.
    Relative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, usage::ValueEnum)]
pub enum ShowIo {
    /// Show I/O for every test case.
    Always,
    /// Show I/O only for failed test cases.
    Failure,
    /// Hide test case I/O.
    Never,
}

#[derive(Debug, usage::Args)]
pub struct Test {
    #[usage(flatten)]
    pub program: ProgramArgs,
    /// Show case input, expected output, and actual output (or interactive transcript).
    #[usage(long, value_enum, default = "failure")]
    pub show_io: ShowIo,
    /// Directory containing .in and .out test files.
    ///
    /// Defaults to the source directory's test subdirectory, or ./test for a direct command.
    #[usage(long, short = 'd')]
    pub test_dir: Option<PathBuf>,
    /// Wall-clock limit per case, in milliseconds.
    #[usage(long, short = 't')]
    pub time_limit: Option<NonZeroU64>,
    /// Sampled process-group RSS limit, in MiB.
    #[usage(long, short = 'm')]
    pub memory_limit: Option<NonZeroU64>,
    /// Ignore trailing spaces and tabs on each line and whitespace at the end of output.
    #[usage(long, short = 's', default = "false")]
    pub strip: bool,
    /// Treat CRLF and LF line endings as equal; disable with --no-ignore-line-ending.
    #[usage(
        long,
        default = "true",
        negate = "--no-ignore-line-ending",
        short = 'l'
    )]
    pub ignore_line_ending: bool,
    /// Stop testing after the first failed case.
    #[usage(long, short = 'f')]
    pub fast_fail: bool,
    /// Allow this nonnegative error when comparing numeric output tokens.
    ///
    /// Nonnumeric tokens must still match exactly. Select the error comparison with --float-error-type.
    #[usage(
        long,
        short = 'e',
        validate = "float(value) >= 0 && float(value) <= 1.7976931348623157e308",
        validate_error = "must be finite and nonnegative"
    )]
    pub float_error: Option<f64>,
    /// Error comparison used with --float-error; both accepts either absolute or relative error.
    #[usage(long, value_enum, default = "both")]
    pub float_error_type: FloatErrorType,
    /// Judge source file, executable file, or shell command.
    #[usage(long, short = 'j')]
    pub judge: Option<String>,
    /// Connect the solution and judge through stdin/stdout for interactive testing; requires --judge.
    #[usage(long, requires("--judge"))]
    pub interactive: bool,
}

#[derive(Debug, usage::Args)]
pub struct Generate {
    #[usage(flatten)]
    pub program: ProgramArgs,
    /// Directory containing generated inputs and answers.
    #[usage(long, default = "random", short = 'd')]
    pub dir: PathBuf,
    /// Generate missing .out files using a reference solution.
    #[usage(long, conflicts("--count"), short = 'a')]
    pub answer: bool,
    /// Number of new test inputs to generate; cannot be combined with --answer.
    #[usage(long, default = "100", short = 'c')]
    pub count: NonZeroUsize,
}

#[derive(Debug, usage::Args)]
pub struct Submit {
    /// Solution source file to submit.
    pub file: PathBuf,
    /// Submit to this problem URL instead of detecting it from the source's .cpg.toml.
    #[usage(long)]
    pub problem: Option<Url>,
    /// Server language ID, overriding the language configuration.
    #[usage(long, short = 'l')]
    pub language: Option<String>,
    /// Allow submitting a file identical to its downloaded template.
    #[usage(long)]
    pub allow_submit_unchanged_solution: bool,
}

#[derive(Debug, usage::Args)]
pub struct Results {
    /// Monitor submissions in an interactive terminal UI.
    #[usage(long, short = 'i')]
    pub ui: bool,
    /// Maximum number of recent submissions to display.
    #[usage(long, default = "20", short = 'n')]
    pub limit: NonZeroUsize,
}

#[derive(Debug, usage::Args)]
pub struct List {
    #[usage(arg_group)]
    pub mode: Option<ListMode>,
}

#[derive(Debug, Clone, Copy, usage::ArgGroup)]
#[usage(name = "kind")]
pub enum ListMode {
    /// List contests and standalone problems (default).
    Workspace,
    /// List contests.
    Contests,
    /// List standalone problems.
    Problems,
    /// List standalone problems and problems within contests.
    AllProblems,
}
