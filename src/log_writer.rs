use colored::Colorize;
use std::io::IsTerminal;
use tracing_log::NormalizeEvent;
use tracing_subscriber::fmt::FormatFields;

pub struct LogFormatter;

// Keep the same level markers, target, and span layout as aviutl2-cli.
impl<S, N> tracing_subscriber::fmt::FormatEvent<S, N> for LogFormatter
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        let normalized = event.normalized_metadata();
        let metadata = match &normalized {
            Some(metadata) => metadata,
            None => event.metadata(),
        };
        let prefix = match *metadata.level() {
            tracing::Level::TRACE => "=)".bright_black(),
            tracing::Level::DEBUG => "-)".bright_magenta(),
            tracing::Level::INFO => "i)".bright_blue(),
            tracing::Level::WARN => "!)".bright_yellow(),
            tracing::Level::ERROR => "x)".bright_red(),
        };
        write!(
            writer,
            "{} {} ",
            prefix,
            format!("[{}]", metadata.target()).bright_black()
        )?;
        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                write!(
                    writer,
                    "{}{}",
                    "<".bright_black(),
                    span.name().bright_black()
                )?;
                let extensions = span.extensions();
                let fields = extensions
                    .get::<tracing_subscriber::fmt::FormattedFields<N>>()
                    .expect("formatted span fields");
                if !fields.is_empty() {
                    write!(
                        writer,
                        "{}{}{}",
                        "{".bright_black(),
                        fields,
                        "}".bright_black()
                    )?;
                }
                write!(writer, "{} ", ">".bright_black())?;
            }
        }
        ctx.format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

pub fn init(no_color: bool) {
    let color =
        !no_color && std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal();
    colored::control::set_override(color);
    tracing_subscriber::fmt()
        .with_ansi(color)
        .event_format(LogFormatter)
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::INFO)
        .init();
}
