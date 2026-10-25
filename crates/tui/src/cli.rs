//! OS-string arguments preserve caller-selected Linux destination bytes.
use duplicity_s3::S3Config;
use std::{ffi::OsString, path::PathBuf, sync::Arc};
#[derive(Debug)]
pub struct Destination {
    pub parent: PathBuf,
    pub name: Vec<u8>,
}
pub enum Source {
    Local(PathBuf),
    S3(S3Config),
}
pub struct Config {
    pub source: Source,
    pub destination: Option<Arc<Destination>>,
}
pub const HELP:&str="Usage: duplicity-tui local DIRECTORY [--restore-parent PARENT --restore-name LEAF]\n       duplicity-tui s3 BUCKET [PREFIX] [--region REGION] [--profile PROFILE] [--endpoint URL] [--path-style] [--restore-parent PARENT --restore-name LEAF]\n\nPlain/gzip history and snapshot browsing. Linux restoration requires r preview, type restore, then Enter; destination must be new. p remains read-only. OpenPGP unsupported. Raw OS paths are preserved; displayed escapes are not input syntax. No source writes or overwrite option.";
//= docs/parsing-requirements.md#restore-ui-001
//= type=implementation
//# The frontend MUST accept a restoration destination only as a complete pair of `--restore-parent PATH` and `--restore-name NAME`, preserving their original OS bytes and rejecting duplicate, missing, oversized or invalid destination values before starting restoration.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Option<Config>, &'static str> {
    let mut args = args.into_iter().peekable();
    let Some(mode) = args.next() else {
        return Ok(None);
    };
    if mode == "--help" && args.peek().is_none() {
        return Ok(None);
    }
    let operand = args.next().ok_or("missing source operand; use --help")?;
    let mut source = match mode.to_str() {
        Some("local") => Source::Local(PathBuf::from(operand)),
        Some("s3") => {
            let bucket = operand
                .into_string()
                .map_err(|_| "S3 bucket requires UTF-8")?;
            let mut config = S3Config::new(bucket);
            if args
                .peek()
                .is_some_and(|arg| arg.to_str().is_some_and(|s| !s.starts_with("--")))
            {
                config.prefix = args
                    .next()
                    .unwrap()
                    .into_string()
                    .map_err(|_| "S3 prefix requires UTF-8")?;
            }
            Source::S3(config)
        }
        _ => return Err("invalid source mode; use --help"),
    };
    #[cfg(target_os = "linux")]
    let (mut parent, mut name): (Option<PathBuf>, Option<Vec<u8>>) = (None, None);
    #[cfg(not(target_os = "linux"))]
    let (parent, name): (Option<PathBuf>, Option<Vec<u8>>) = (None, None);
    while let Some(option) = args.next() {
        let option = option.to_str().ok_or("option token requires UTF-8")?;
        if option == "--path-style" {
            if let Source::S3(config) = &mut source {
                config.force_path_style = true;
                continue;
            }
            return Err("S3 option unavailable for local source");
        }
        if !matches!(
            option,
            "--restore-parent" | "--restore-name" | "--region" | "--profile" | "--endpoint"
        ) {
            return Err("unknown option; use --help");
        }
        let value = args.next().ok_or("missing option value")?;
        match option {
            "--restore-parent" => {
                #[cfg(target_os = "linux")]
                {
                    if parent.is_some() {
                        return Err("duplicate restore-parent");
                    }
                    #[cfg(unix)]
                    {
                        use std::os::unix::ffi::OsStrExt;
                        if value.as_os_str().as_bytes().is_empty()
                            || value.as_os_str().as_bytes().contains(&0)
                        {
                            return Err("restore parent must be nonempty without NUL");
                        }
                        if value.as_os_str().as_bytes().len() > 65536 {
                            return Err("restore parent exceeds byte limit");
                        }
                    }
                    parent = Some(PathBuf::from(value));
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let _ = value;
                    return Err("native restoration unsupported on this OS");
                }
            }
            "--restore-name" => {
                #[cfg(target_os = "linux")]
                {
                    if name.is_some() {
                        return Err("duplicate restore-name");
                    }
                    use std::os::unix::ffi::OsStringExt;
                    let bytes = value.into_vec();
                    if bytes.len() > 4096
                        || bytes.is_empty()
                        || bytes == b"."
                        || bytes == b".."
                        || bytes.contains(&0)
                        || bytes.contains(&b'/')
                    {
                        return Err("restore name must be one bounded raw component");
                    }
                    name = Some(bytes);
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let _ = value;
                    return Err("native restoration unsupported on this OS");
                }
            }
            _ => {
                let Source::S3(config) = &mut source else {
                    return Err("S3 option unavailable for local source");
                };
                let text = value
                    .into_string()
                    .map_err(|_| "S3 option value requires UTF-8")?;
                match option {
                    "--region" => config.region = Some(text),
                    "--profile" => config.profile = Some(text),
                    "--endpoint" => config.endpoint = Some(text),
                    _ => unreachable!(),
                }
            }
        }
    }
    let destination = match (parent, name) {
        (None, None) => None,
        (Some(parent), Some(name)) => Some(Arc::new(Destination { parent, name })),
        _ => return Err("restore-parent and restore-name must be provided together"),
    };
    Ok(Some(Config {
        source,
        destination,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "linux")]
    #[test]
    fn pairs_duplicates_and_values_beginning_with_flags() {
        let parsed = parse(
            [
                "local",
                "source",
                "--restore-parent",
                "--name",
                "--restore-name",
                "--leaf",
            ]
            .map(OsString::from),
        )
        .unwrap()
        .unwrap();
        assert_eq!(parsed.destination.unwrap().name, b"--leaf");
        for args in [
            vec!["local", "s", "--restore-parent", "p"],
            vec!["local", "s", "--restore-name", "n"],
            vec![
                "local",
                "s",
                "--restore-parent",
                "p",
                "--restore-parent",
                "p",
            ],
            vec!["local", "s", "--restore-name"],
            vec!["local", "s", "--unknown", "v"],
            vec!["local", "s", "--restore-name", "../x"],
        ] {
            assert!(parse(args.into_iter().map(OsString::from)).is_err());
        }
    }
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn native_restore_options_are_rejected_on_unsupported_os() {
        assert!(matches!(
            parse(["local", "source", "--restore-parent", "parent"].map(OsString::from)),
            Err("native restoration unsupported on this OS")
        ));
        assert!(matches!(
            parse(["local", "source", "--restore-name", "name"].map(OsString::from)),
            Err("native restoration unsupported on this OS")
        ));
    }
    #[cfg(target_os = "linux")]
    #[test]
    //= docs/parsing-requirements.md#restore-ui-001
    //= type=test
    //# The frontend MUST accept a restoration destination only as a complete pair of `--restore-parent PATH` and `--restore-name NAME`, preserving their original OS bytes and rejecting duplicate, missing, oversized or invalid destination values before starting restoration.
    fn parent_empty_nul_and_exact_byte_caps() {
        for (size, ok) in [(65536, true), (65537, false)] {
            let parsed = parse(vec![
                "local".into(),
                "s".into(),
                "--restore-name".into(),
                "n".into(),
                "--restore-parent".into(),
                OsString::from("p".repeat(size)),
            ]);
            assert_eq!(parsed.is_ok(), ok);
        }
        for value in ["", "a\0b"] {
            assert!(parse(
                [
                    "local",
                    "s",
                    "--restore-name",
                    "n",
                    "--restore-parent",
                    value
                ]
                .map(OsString::from)
            )
            .is_err());
        }
    }
    #[cfg(target_os = "linux")]
    #[cfg(target_os = "linux")]
    #[test]
    //= docs/parsing-requirements.md#restore-ui-001
    //= type=test
    //# The frontend MUST accept a restoration destination only as a complete pair of `--restore-parent PATH` and `--restore-name NAME`, preserving their original OS bytes and rejecting duplicate, missing, oversized or invalid destination values before starting restoration.
    fn raw_os_paths_and_exact_bounds() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let args = vec![
            OsString::from("local"),
            OsString::from_vec(b"source\xff".to_vec()),
            "--restore-parent".into(),
            OsString::from_vec(b"parent\xff".to_vec()),
            "--restore-name".into(),
            OsString::from_vec(b"name\xff".to_vec()),
        ];
        let config = parse(args).unwrap().unwrap();
        assert_eq!(
            config.destination.unwrap().parent.as_os_str().as_bytes(),
            b"parent\xff"
        );
        for (size, ok) in [(4096, true), (4097, false)] {
            assert_eq!(
                parse(vec![
                    "local".into(),
                    "s".into(),
                    "--restore-parent".into(),
                    "p".into(),
                    "--restore-name".into(),
                    OsString::from_vec(vec![b'n'; size])
                ])
                .is_ok(),
                ok
            );
        }
    }
}
