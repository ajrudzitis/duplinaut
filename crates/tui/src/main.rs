use duplicity_core::source::{Cancellation, ObjectSource};
use duplicity_s3::S3Source;
use std::{
    io::{self, IsTerminal, Write},
    sync::Arc,
};
fn main() {
    if let Err(error) = run() {
        let safe = duplicity_tui::display::bytes(error.to_string().as_bytes());
        let _ = writeln!(io::stderr(), "duplicity-tui: {safe}");
        std::process::exit(2);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Some(config) =
        duplicity_tui::cli::parse(std::env::args_os().skip(1)).map_err(io::Error::other)?
    else {
        writeln!(io::stdout(), "{}", duplicity_tui::cli::HELP)?;
        return Ok(());
    };
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("interactive browsing requires a terminal".into());
    }
    let cancel = Cancellation::default();
    let (source, prefix): (Arc<dyn ObjectSource>, String) = match config.source {
        #[cfg(unix)]
        duplicity_tui::cli::Source::Local(path) => (
            Arc::new(duplicity_core::local::LocalSource::new(path)?),
            String::new(),
        ),
        #[cfg(not(unix))]
        duplicity_tui::cli::Source::Local(_) => {
            return Err("local source unsupported on this OS".into())
        }
        duplicity_tui::cli::Source::S3(config) => {
            let prefix = config.prefix.clone();
            (Arc::new(S3Source::connect(config, &cancel)?), prefix)
        }
    };
    duplicity_tui::browser::run_configured(source, prefix, cancel, config.destination)?;
    Ok(())
}
