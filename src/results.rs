use crate::{
    cli::Results,
    config::Paths,
    model::{Metadata, Submission, SubmissionScope},
    services::Services,
};
use anyhow::{Result, ensure};
use console::{Alignment, Style, measure_text_width, pad_str, truncate_str};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, IsTerminal, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

const REFRESH_INTERVAL: Duration = Duration::from_secs(3);
const HEADERS: [&str; 6] = ["STATUS", "PROBLEM", "RUNTIME", "LANGUAGE", "TIME", "ID"];

pub fn run(
    args: &Results,
    paths: Paths,
    scope: SubmissionScope,
    no_color: bool,
    interrupted: &AtomicBool,
) -> Result<()> {
    let terminal = io::stdout().is_terminal();
    let color = terminal && !no_color && std::env::var_os("NO_COLOR").is_none();
    if args.ui && terminal {
        ensure!(io::stdin().is_terminal(), "--ui requires terminal input");
        return monitor(paths, scope, args.limit.get(), color, interrupted);
    }
    ensure!(!interrupted.load(Ordering::Relaxed), "Interrupted");
    let url = match &scope {
        Metadata::Problem { reference, .. } => &reference.url,
        Metadata::Contest(contest) => &contest.reference.url,
    };
    tracing::info!("Fetching up to {} submissions for {url}...", args.limit);
    let services = Services::new(&paths)?;
    let submissions = services
        .backend(scope.service())
        .submissions(&scope, args.limit.get())?;
    tracing::info!("Fetched {} submission(s)", submissions.len());
    let mut output = io::stdout().lock();
    if terminal {
        for (index, line) in table(&submissions, color).iter().enumerate() {
            writeln!(output, "  {line}")?;
            if index > 0 {
                writeln!(output, "    {}", submissions[index - 1].url)?;
            }
        }
    } else {
        // Preserve the TSV contract, including column order and empty fields.
        writeln!(output, "ID\tTIME\tPROBLEM\tLANGUAGE\tSTATUS\tRUNTIME\tURL")?;
        for result in submissions {
            writeln!(
                output,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                result.id,
                result.submitted_at,
                result.problem_id,
                result.language,
                result.status,
                result.time,
                result.url
            )?;
        }
    }
    Ok(())
}

pub(crate) fn color_status(status: &str, color: bool) -> String {
    if !color {
        return status.to_owned();
    }
    // ANSI foreground approximations of AtCoder Error Colorizer by iiko11 (MIT):
    // https://greasyfork.org/scripts/478281-atcoder-error-colorizer
    let code = match status.split_whitespace().last() {
        Some("AC") => 32,  // green
        Some("WA") => 33,  // yellow
        Some("TLE") => 95, // bright magenta
        Some("RE") => 35,  // magenta
        Some("CE") => 94,  // bright blue
        Some("MLE") => 31, // red
        Some("OLE") => 91, // bright red
        Some("IE") => 36,  // cyan
        _ => 90,           // bright black
    };
    format!("\x1b[{code}m{status}\x1b[0m")
}

fn table(submissions: &[Submission], color: bool) -> Vec<String> {
    let mut rows = vec![HEADERS.map(str::to_owned)];
    rows.extend(submissions.iter().map(|s| {
        [
            &s.status,
            &s.problem_id,
            &s.time,
            &s.language,
            &s.submitted_at,
            &s.id,
        ]
        // Terminal control characters must not alter the table or screen.
        .map(|s| s.chars().filter(|c| !c.is_control()).collect::<String>())
    }));
    let widths: [usize; 6] = std::array::from_fn(|column| {
        rows.iter()
            .map(|row| measure_text_width(&row[column]))
            .max()
            .expect("header row")
    });
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            row.iter()
                .enumerate()
                .map(|(column, text)| {
                    let alignment = if column < 2 {
                        Alignment::Left
                    } else {
                        Alignment::Right
                    };
                    let padded = pad_str(text, widths[column], alignment, None);
                    match (index, column) {
                        (0, _) => Style::new()
                            .bold()
                            .apply_to(padded)
                            .force_styling(color)
                            .to_string(),
                        (_, 0) => color_status(&padded, color),
                        _ => padded.into_owned(),
                    }
                })
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect()
}

struct Screen;

impl Screen {
    fn enter() -> Result<Self> {
        terminal::enable_raw_mode()?;
        let screen = Self;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(screen)
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn monitor(
    paths: Paths,
    scope: SubmissionScope,
    limit: usize,
    color: bool,
    interrupted: &AtomicBool,
) -> Result<()> {
    let title = match &scope {
        Metadata::Contest(contest) => &contest.title,
        Metadata::Problem { title, .. } => title,
    };
    let title = format!("cpg results — {title}");
    let title: String = title.chars().filter(|c| !c.is_control()).collect();
    let (request_tx, request_rx) = mpsc::channel();
    let (update_tx, update_rx) = mpsc::channel();
    // Keep keyboard handling and terminal restoration responsive during HTTP requests.
    thread::spawn(move || {
        let services = match Services::new(&paths) {
            Ok(services) => services,
            Err(error) => {
                let _ = update_tx.send(Err(error));
                return;
            }
        };
        while request_rx.recv().is_ok() {
            let result = services.backend(scope.service()).submissions(&scope, limit);
            if update_tx.send(result).is_err() {
                break;
            }
        }
    });
    let (open_tx, open_rx) = mpsc::channel();
    let _screen = Screen::enter()?;
    let mut fetching = false;
    let mut paused = false;
    let mut refresh_at = Instant::now();
    let mut submissions = Vec::new();
    let mut rows = table(&submissions, color);
    let mut offset = 0usize;
    let mut message = String::new();
    let mut previous_frame = Vec::new();
    let animation_started = Instant::now();
    while !interrupted.load(Ordering::Relaxed) {
        match update_rx.try_recv() {
            Ok(result) => {
                submissions = result?;
                rows = table(&submissions, color);
                fetching = false;
                refresh_at = Instant::now() + REFRESH_INTERVAL;
            }
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => anyhow::bail!("Submission monitor stopped"),
        }
        if let Ok(result) = open_rx.try_recv() {
            message = match result {
                Ok(()) => "Opened submission in browser".into(),
                Err(error) => format!("{error:#}"),
            };
        }
        if !paused && !fetching && Instant::now() >= refresh_at {
            request_tx.send(())?;
            fetching = true;
        }
        let (width, height) = terminal::size()?;
        let visible = usize::from(height.saturating_sub(3));
        offset = offset.min(submissions.len().saturating_sub(visible.max(1)));
        let mut frame = Vec::new();
        execute!(frame, Clear(ClearType::All))?;
        let mut line = |y: u16, text: &str| -> Result<()> {
            if y < height && width > 1 {
                execute!(frame, MoveTo(0, y))?;
                write!(
                    frame,
                    "{}",
                    truncate_str(text, usize::from(width.saturating_sub(1)), "…")
                )?;
            }
            Ok(())
        };
        line(
            0,
            &Style::new()
                .bold()
                .apply_to(&title)
                .force_styling(color)
                .to_string(),
        )?;
        if height >= 3 {
            line(1, &format!("    {}", rows[0]))?;
            if submissions.is_empty() && visible > 0 {
                line(
                    2,
                    if fetching {
                        "  Loading submissions…"
                    } else {
                        "  No submissions"
                    },
                )?;
            }
            for (index, row) in rows.iter().skip(1 + offset).take(visible).enumerate() {
                let key = if index < 10 {
                    format!("[{}] ", (index + 1) % 10)
                } else {
                    "    ".into()
                };
                line(height - 2 - index as u16, &format!("{key}{row}"))?;
            }
        }
        let (indicator, state) = if paused {
            ('*', "Paused")
        } else if fetching {
            ('!', "Refreshing")
        } else {
            let frame = (animation_started.elapsed().as_millis() / 100 % 4) as usize;
            (['|', '/', '-', '\\'][frame], "Running")
        };
        line(
            height.saturating_sub(1),
            &format!(
                "{indicator} {state:<10} | p: pause/resume  r: refresh  ↑/↓: scroll  1–0: open  q: quit | {} submissions {message}",
                submissions.len()
            ),
        )?;
        if frame != previous_frame {
            let mut output = io::stdout().lock();
            output.write_all(&frame)?;
            output.flush()?;
            previous_frame = frame;
        }
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                interrupted.store(true, Ordering::Relaxed);
                break;
            }
            KeyCode::Char('q') | KeyCode::Esc => break,
            KeyCode::Char('p') => paused = !paused,
            KeyCode::Char('r') if !fetching => {
                request_tx.send(())?;
                fetching = true;
            }
            KeyCode::Up | KeyCode::Char('k') => offset = offset.saturating_add(1),
            KeyCode::Down | KeyCode::Char('j') => offset = offset.saturating_sub(1),
            KeyCode::Char(key @ '0'..='9') => {
                let index = if key == '0' {
                    9
                } else {
                    key as usize - '1' as usize
                };
                if let Some(submission) =
                    submissions.get(offset + index).filter(|_| index < visible)
                {
                    let url = submission.url.clone();
                    let sender = open_tx.clone();
                    message = "Opening submission…".into();
                    thread::spawn(move || {
                        let _ = sender.send(crate::open_browser(&url));
                    });
                }
            }
            _ => (),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_and_unicode_columns() {
        for (status, color) in [
            ("AC", 32),
            ("WA", 33),
            ("TLE", 95),
            ("RE", 35),
            ("CE", 94),
            ("MLE", 31),
            ("OLE", 91),
            ("IE", 36),
            ("WJ", 90),
            ("WR", 90),
            ("1 / 20", 90),
            ("unknown", 90),
        ] {
            let text = color_status(status, true);
            assert_eq!(text, format!("\x1b[{color}m{status}\x1b[0m"));
            assert_eq!(console::strip_ansi_codes(&text), status);
            assert_eq!(color_status(status, false), status);
        }
        let first = Submission {
            id: "10".into(),
            url: "https://mock.local/submissions/10".parse().unwrap(),
            problem_id: "問題Ａ".into(),
            submitted_at: "2026-09-05".into(),
            language: "C++ (GCC 15)".into(),
            status: "AC".into(),
            time: "1 ms".into(),
        };
        let second = Submission {
            problem_id: "B\n\t\x1b".into(),
            status: "TLE".into(),
            ..first.clone()
        };
        let plain = table(&[first.clone(), second.clone()], false);
        let colored = table(&[first, second], true);
        for (plain, colored) in plain.iter().zip(&colored) {
            assert_eq!(console::strip_ansi_codes(colored), plain.as_str());
            assert!(!plain.chars().any(char::is_control));
        }
        let column_ends: Vec<_> = plain
            .iter()
            .zip(["RUNTIME", "1 ms", "1 ms"])
            .map(|(line, cell)| {
                let end = line.find(cell).unwrap() + cell.len();
                measure_text_width(&line[..end])
            })
            .collect();
        assert_eq!(column_ends, vec![column_ends[0]; 3]);
        assert_eq!(table(&[], false).len(), 1);
    }
}
