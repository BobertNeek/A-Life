//! Native, read-only inspection. This executable never creates a GPU session.
use std::{ffi::OsString, path::PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, PartialEq, Eq)]
enum InspectionRequest {
    Archive {
        source: PathBuf,
        manifest: PathBuf,
    },
    Checkpoint {
        source: PathBuf,
        expected_policy: u64,
    },
}

fn parse_request(mut args: impl Iterator<Item = OsString>) -> Result<InspectionRequest> {
    let first = args.next().ok_or(
        "usage: inspect_archived_success SEALED_CYCLE MANIFEST | --validate-checkpoint SEALED_CYCLE --expected-policy POLICY",
    )?;
    if first == "--validate-checkpoint" {
        let source = PathBuf::from(args.next().ok_or("missing checkpoint directory")?);
        if args.next().as_deref() != Some(std::ffi::OsStr::new("--expected-policy")) {
            return Err("checkpoint inspection requires --expected-policy POLICY".into());
        }
        let policy = args.next().ok_or("missing expected policy")?;
        let expected_policy = policy
            .to_str()
            .ok_or("expected policy must be an unsigned integer")?
            .parse()?;
        if args.next().is_some() {
            return Err("unexpected checkpoint inspection argument".into());
        }
        return Ok(InspectionRequest::Checkpoint {
            source,
            expected_policy,
        });
    }
    let source = PathBuf::from(first);
    let manifest = PathBuf::from(args.next().ok_or("missing archived success manifest")?);
    if args.next().is_some() {
        return Err("unexpected archive inspection argument".into());
    }
    Ok(InspectionRequest::Archive { source, manifest })
}

fn main() -> Result<()> {
    let request = parse_request(std::env::args_os().skip(1))?;
    // Native replay records contain large fixed-size state frames. Keep their
    // read-only decoding off Windows' small default executable stack.
    let inspection = std::thread::Builder::new()
        .name("archive-inspection".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || -> std::result::Result<String, String> {
            match request {
                InspectionRequest::Archive { source, manifest } => {
                    let inspection = alife_game_app::inspect_archived_successes(&source, &manifest)
                        .map_err(|error| error.to_string())?;
                    serde_json::to_string_pretty(&inspection).map_err(|error| error.to_string())
                }
                InspectionRequest::Checkpoint {
                    source,
                    expected_policy,
                } => {
                    let inspection =
                        alife_game_app::inspect_foundation_checkpoint(&source, expected_policy)
                            .map_err(|error| error.to_string())?;
                    serde_json::to_string_pretty(&inspection).map_err(|error| error.to_string())
                }
            }
        })?
        .join()
        .map_err(|_| "native archive inspection worker panicked")??;
    println!("{inspection}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<InspectionRequest> {
        parse_request(args.iter().map(OsString::from))
    }

    #[test]
    fn original_archive_cli_is_preserved() {
        assert_eq!(
            parse(&["source", "manifest"]).unwrap(),
            InspectionRequest::Archive {
                source: PathBuf::from("source"),
                manifest: PathBuf::from("manifest"),
            }
        );
        for args in [
            &[][..],
            &["source"][..],
            &["source", "manifest", "extra"][..],
        ] {
            assert!(parse(args).is_err());
        }
    }

    #[test]
    fn checkpoint_cli_requires_one_explicit_policy_and_no_archive_arguments() {
        assert_eq!(
            parse(&[
                "--validate-checkpoint",
                "source",
                "--expected-policy",
                "3238"
            ])
            .unwrap(),
            InspectionRequest::Checkpoint {
                source: PathBuf::from("source"),
                expected_policy: 3238,
            }
        );
        for args in [
            &["--validate-checkpoint"][..],
            &["--validate-checkpoint", "source"][..],
            &["--validate-checkpoint", "source", "--expected-policy"][..],
            &["--validate-checkpoint", "source", "manifest"][..],
            &[
                "--validate-checkpoint",
                "source",
                "--expected-policy",
                "nope",
            ][..],
            &["--validate-checkpoint", "source", "--expected-policy", "-1"][..],
            &[
                "--validate-checkpoint",
                "source",
                "--expected-policy",
                "18446744073709551616",
            ][..],
            &[
                "--validate-checkpoint",
                "source",
                "--expected-policy",
                "3238",
                "manifest",
            ][..],
            &[
                "--validate-checkpoint",
                "source",
                "--expected-policy",
                "3238",
                "--expected-policy",
                "3237",
            ][..],
            &["source", "manifest", "--expected-policy", "3238"][..],
        ] {
            assert!(parse(args).is_err(), "{args:?}");
        }
    }
}
