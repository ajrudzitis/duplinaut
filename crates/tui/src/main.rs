use duplicity_core::source::{Cancellation, ObjectSource};
use duplicity_s3::{S3Config, S3Source};
use std::{io::IsTerminal, sync::Arc};

fn main() {
    if let Err(error) = run() {
        eprintln!("duplicity-tui: {error}");
        std::process::exit(2);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args == ["--help"] {
        println!("Usage: duplicity-tui local DIRECTORY\n       duplicity-tui s3 BUCKET [PREFIX] [--region REGION] [--profile PROFILE] [--endpoint URL] [--path-style]\n\nPlain/gzip history and snapshot browsing. OpenPGP and filesystem restoration are under construction.");
        return Ok(());
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err("interactive browsing requires a terminal".into());
    }
    let cancel = Cancellation::default();
    let (source, prefix): (Arc<dyn ObjectSource>, String) = match args[0].as_str() {
        #[cfg(unix)]
        "local" if args.len() == 2 => (
            Arc::new(duplicity_core::local::LocalSource::new(&args[1])?),
            String::new(),
        ),
        "s3" if args.len() >= 2 => {
            let mut config = S3Config::new(&args[1]);
            let mut index = 2;
            if args.get(index).is_some_and(|arg| !arg.starts_with("--")) {
                config.prefix = args[index].clone();
                index += 1;
            }
            while let Some(option) = args.get(index) {
                if option == "--path-style" {
                    config.force_path_style = true;
                    index += 1;
                    continue;
                }
                let value = args.get(index + 1).ok_or("missing S3 option value")?;
                match option.as_str() {
                    "--region" => config.region = Some(value.clone()),
                    "--profile" => config.profile = Some(value.clone()),
                    "--endpoint" => config.endpoint = Some(value.clone()),
                    _ => return Err("unknown S3 option".into()),
                }
                index += 2;
            }
            let prefix = config.prefix.clone();
            (Arc::new(S3Source::connect(config, &cancel)?), prefix)
        }
        _ => return Err("invalid arguments; use --help".into()),
    };
    duplicity_tui::browser::run(source, prefix, cancel)?;
    Ok(())
}
