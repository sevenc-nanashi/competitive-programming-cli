#[cfg(not(target_os = "linux"))]
compile_error!("cpg currently supports Linux only");

mod cli;
mod config;
mod log_writer;
mod model;
mod results;
mod runner;
mod services;
mod workspace;

use anyhow::{Context, Result, bail, ensure};
use cli::{Cli, Commands, ConfigField, ListMode};
use config::{Config, Paths, expand_path};
use model::{Metadata, ResourceRef, ServiceId, SubmissionRequest};
use services::Services;
use std::{
    fs,
    sync::atomic::{AtomicBool, Ordering},
};

fn open_browser(url: &url::Url) -> Result<()> {
    open::that(url.as_str()).with_context(|| format!("Cannot open {url} in your browser"))
}

fn run(cli: Cli, interrupted: &AtomicBool) -> Result<bool> {
    let paths = Paths::discover()?;
    match cli.command {
        Commands::Completion(args) => {
            let shell = usage::complete::Shell::from_name(&args.shell)
                .context("Unsupported completion shell")?;
            print!("{}", Cli::completion_script(shell));
        }
        Commands::Init(args) => config::init(&paths, &args, interrupted)?,
        Commands::Config(args) => {
            let config_dir = std::path::absolute(&paths.config)?;
            match args.field {
                Some(field) => {
                    let path = match field {
                        ConfigField::Schema => {
                            println!("{}", serde_json::to_string_pretty(&Config::schema())?);
                            return Ok(true);
                        }
                        ConfigField::Root => Config::load(&paths)?.root()?,
                        ConfigField::ConfigDir => config_dir,
                        ConfigField::CookiesDir => std::path::absolute(&paths.cookies)?,
                        ConfigField::WorkspaceTemplateDir => config_dir.join("workspace_template"),
                        ConfigField::ProblemTemplateDir => config_dir.join("problem_template"),
                        ConfigField::ContestTemplateDir => config_dir.join("contest_template"),
                        ConfigField::SingleProblemTemplateDir => {
                            config_dir.join("single_problem_template")
                        }
                    };
                    println!("{}", path.display());
                }
                None => {
                    let style = console::Style::new().blue().bright().bold();
                    for (label, path) in [
                        ("Workspace root:", Config::load(&paths)?.root()?),
                        ("Configuration directory:", config_dir.clone()),
                        ("Cookies directory:", std::path::absolute(&paths.cookies)?),
                        (
                            "Workspace template directory:",
                            config_dir.join("workspace_template"),
                        ),
                        (
                            "Problem template directory:",
                            config_dir.join("problem_template"),
                        ),
                        (
                            "Contest template directory:",
                            config_dir.join("contest_template"),
                        ),
                        (
                            "Single problem template directory:",
                            config_dir.join("single_problem_template"),
                        ),
                    ] {
                        println!("{} {}", style.apply_to(label), path.display());
                    }
                }
            }
        }
        Commands::Login(args) => Services::login(&paths, args.service, &args.cookie_file)?,
        Commands::Test(args) => {
            return runner::test(&Config::load(&paths)?, &args, interrupted);
        }
        Commands::Generate(args) => runner::generate(&Config::load(&paths)?, &args, interrupted)?,
        Commands::Open(args) => {
            let url = match workspace::locate(&std::env::current_dir()?)? {
                Metadata::Problem { reference, .. } => reference.url,
                Metadata::Contest(contest) => contest.reference.url,
            };
            ServiceId::from_url(&url)?;
            ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
            if args.url_only {
                println!("{url}");
            } else {
                tracing::info!("Opening {url} in your browser...");
                open_browser(&url)?;
                tracing::info!("Opened {url}");
            }
        }
        Commands::List(args) => {
            let config = Config::load(&paths)?;
            let root = config.root()?;
            let mode = match args.mode {
                Some(mode) => mode,
                None => ListMode::Workspace,
            };
            for path in workspace::list(&config, mode)? {
                println!("{}", path.strip_prefix(&root)?.display());
            }
        }
        Commands::Download(args) => {
            let services = Services::new(&paths)?;
            let problem = services.resolve(&args.url)?.problem()?;
            let directory = workspace::download(
                &paths,
                &Config::load(&paths)?,
                &services,
                ResourceRef::Problem(problem),
                interrupted,
            )?;
            println!("{}", directory.display());
        }
        Commands::Prepare(args) => {
            let services = Services::new(&paths)?;
            let contest = services.resolve(&args.url)?.contest()?;
            let directory = workspace::download(
                &paths,
                &Config::load(&paths)?,
                &services,
                ResourceRef::Contest(contest),
                interrupted,
            )?;
            println!("{}", directory.display());
        }
        Commands::Submit(args) => {
            let config = Config::load(&paths)?;
            let source_path = fs::canonicalize(expand_path(&args.file)?)?;
            let source = fs::read_to_string(&source_path)?;
            let local = workspace::find_metadata(
                source_path
                    .parent()
                    .context("Source has no parent directory")?,
            )?;
            if !args.clipboard
                && let Some((
                    directory,
                    Metadata::Problem {
                        template_checksums, ..
                    },
                )) = &local
                && let Some(expected) = template_checksums.get(source_path.strip_prefix(directory)?)
                && *expected == workspace::checksum(source.as_bytes())
            {
                tracing::warn!("{} is unchanged from its template", source_path.display());
                ensure!(
                    args.allow_submit_unchanged_solution,
                    "Use --allow-submit-unchanged-solution to submit the unchanged template"
                );
            }
            let configured_language = config.match_language(&source_path)?;
            tracing::info!("Preparing {} for submission...", source_path.display());
            let prepared_source = configured_language
                .map(|language| runner::prepare_source(language, &source_path, true, interrupted))
                .transpose()?
                .flatten();
            let source = match &prepared_source {
                Some(prepared) => fs::read_to_string(prepared.path())?,
                None => source,
            };
            if args.clipboard {
                runner::copy_to_clipboard(&config.clipboard, &source, interrupted)?;
                tracing::info!("Copied {} bytes to the clipboard", source.len());
                return Ok(true);
            }
            let services = Services::new(&paths)?;
            let problem = match args.problem {
                Some(url) => services.resolve(&url)?.problem()?,
                None => match local
                    .context("No .cpg.toml found for the source; specify --problem URL")?
                    .1
                {
                    Metadata::Problem { reference, .. } => reference,
                    Metadata::Contest(_) => bail!("Specify a problem directory or --problem URL"),
                },
            };
            let backend = services.backend(problem.service);
            tracing::info!("Submission target: {}", problem.url);
            let language = match args.language {
                Some(language) => Some(language),
                None => configured_language
                    .and_then(|language| language.submit.get(backend.auth_service().as_str()))
                    .cloned(),
            };
            tracing::info!(
                "Fetching submission languages from {}...",
                backend.auth_service().as_str()
            );
            let languages = backend.languages(&problem)?;
            let Some(language) =
                language.and_then(|id| languages.iter().find(|language| language.id == id))
            else {
                tracing::error!(
                    "Choose a submission language using --language or language.<name>.submit.{}:",
                    backend.auth_service().as_str()
                );
                for language in languages {
                    tracing::info!("{}\t{}", language.id, language.name);
                }
                bail!("Submission language is missing or invalid");
            };
            ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
            tracing::info!(
                "Submitting with language ID {} ({}, {} bytes)...",
                language.id,
                language.name,
                source.len()
            );
            let submission = backend.submit(&SubmissionRequest {
                problem: &problem,
                language: &language.id,
                source: &source,
            })?;
            println!("Submitted {}: {}", submission.id, submission.url);
        }
        Commands::Results(args) => {
            let scope = workspace::locate(&std::env::current_dir()?)?;
            results::run(&args, paths, scope, interrupted)?;
        }
    }
    Ok(true)
}

fn main() {
    let cli = Cli::parse();
    log_writer::init(cli.no_color);
    let interrupted = match runner::install_signal_handler() {
        Ok(interrupted) => interrupted,
        Err(error) => {
            tracing::error!("{error:#}");
            std::process::exit(2);
        }
    };
    let result = run(cli, &interrupted);
    if interrupted.load(Ordering::Relaxed) {
        std::process::exit(130);
    }
    match result {
        Ok(true) => (),
        Ok(false) => std::process::exit(1),
        Err(error) => {
            tracing::error!("{error:#}");
            std::process::exit(2);
        }
    }
}
