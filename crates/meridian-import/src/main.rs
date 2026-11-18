use std::{ffi::OsString, path::PathBuf, process::ExitCode};

use meridian_import::{
    ImportBatch, ImportError, Model, Result, import_all, import_ship, import_station, read_file,
    read_repository,
};

const HELP: &str = "meridian-import: pinned original SH025/SH028 offline conversion

meridian-import ship --source SH025.toml (--check REFERENCE_DIR | --output NEW_DIR)
meridian-import station --ship-source SH025.toml --station-source SH028.toml (--check REFERENCE_DIR | --output NEW_DIR)
meridian-import all --ship-source SH025.toml --station-source SH028.toml (--check REFERENCE_DIR | --output NEW_DIR)
meridian-import (ship|station|all) --repository OBJECT_DATABASE (--check REFERENCE_DIR | --output NEW_DIR)

Only exact a11 reference bytes are accepted. Repository mode reads fixed Git blobs,
never HEAD or working files. --check is read-only against original asset/manifests.
--output creates a new directory and an accurately labelled Rust producer receipt.
Existing destinations and mixed/duplicate arguments refuse. No runtime assets are replaced.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Ship,
    Station,
    All,
}

#[derive(Debug)]
enum Input {
    Repository(PathBuf),
    Files {
        ship: PathBuf,
        station: Option<PathBuf>,
    },
}

#[derive(Debug)]
enum Destination {
    Check(PathBuf),
    New(PathBuf),
}

#[derive(Debug)]
struct Options {
    kind: Kind,
    input: Input,
    destination: Destination,
}

fn usage(message: impl Into<String>) -> ImportError {
    ImportError::Usage(message.into())
}

fn assign<T>(slot: &mut Option<T>, value: T, label: &str) -> Result<()> {
    if slot.replace(value).is_some() {
        return Err(usage(format!("Duplicate {label}")));
    }
    Ok(())
}

fn value(args: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<PathBuf> {
    let result = args
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| usage(format!("{flag} needs a path")))?;
    Ok(result.into())
}

fn options(mut args: impl Iterator<Item = OsString>) -> Result<Options> {
    let kind = match args.next().as_deref().and_then(std::ffi::OsStr::to_str) {
        Some("ship") => Kind::Ship,
        Some("station") => Kind::Station,
        Some("all") => Kind::All,
        _ => return Err(usage("Select ship, station or all; use --help for syntax")),
    };
    let (mut repository, mut ship, mut station, mut destination) = (None, None, None, None);
    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--repository") => assign(
                &mut repository,
                value(&mut args, "--repository")?,
                "repository",
            )?,
            Some("--source") if kind == Kind::Ship => {
                assign(&mut ship, value(&mut args, "--source")?, "ship source")?
            }
            Some("--ship-source") if kind != Kind::Ship => {
                assign(&mut ship, value(&mut args, "--ship-source")?, "ship source")?
            }
            Some("--station-source") if kind != Kind::Ship => assign(
                &mut station,
                value(&mut args, "--station-source")?,
                "station source",
            )?,
            Some("--check") => assign(
                &mut destination,
                Destination::Check(value(&mut args, "--check")?),
                "destination mode",
            )?,
            Some("--output") => assign(
                &mut destination,
                Destination::New(value(&mut args, "--output")?),
                "destination mode",
            )?,
            _ => {
                return Err(usage(format!(
                    "Unknown or inappropriate argument: {}",
                    argument.to_string_lossy()
                )));
            }
        }
    }
    let input = match (repository, ship, station) {
        (Some(repo), None, None) => Input::Repository(repo),
        (None, Some(ship), station) if kind == Kind::Ship || station.is_some() => {
            Input::Files { ship, station }
        }
        _ => {
            return Err(usage(
                "Choose one repository or the complete file-source arguments for the selected mode",
            ));
        }
    };
    let destination = destination.ok_or_else(|| usage("Choose exactly one --check or --output"))?;
    Ok(Options {
        kind,
        input,
        destination,
    })
}

fn convert(options: &Options) -> Result<ImportBatch> {
    let (ship, station) = match &options.input {
        Input::Repository(path) => {
            let ship = read_repository(path, Model::Sh025)?;
            let station = if options.kind == Kind::Ship {
                None
            } else {
                Some(read_repository(path, Model::Sh028)?)
            };
            (ship, station)
        }
        Input::Files { ship, station } => {
            let ship = read_file(ship, Model::Sh025)?;
            let station = station
                .as_ref()
                .map(|path| read_file(path, Model::Sh028))
                .transpose()?;
            (ship, station)
        }
    };
    match options.kind {
        Kind::Ship => import_ship(&ship),
        Kind::Station => import_station(
            &ship,
            station
                .as_deref()
                .ok_or_else(|| usage("Missing station source"))?,
        ),
        Kind::All => import_all(
            &ship,
            station
                .as_deref()
                .ok_or_else(|| usage("Missing station source"))?,
        ),
    }
}

fn run(options: Options) -> Result<()> {
    let batch = convert(&options)?;
    match options.destination {
        Destination::Check(path) => {
            batch.check_reference_directory(&path)?;
            println!("Original selected assets, manifests and notice match");
        }
        Destination::New(path) => {
            batch.write_new_directory(&path)?;
            println!(
                "Wrote {} verified binary assets and Rust receipt to {}",
                batch.artifacts().len(),
                path.display()
            );
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() == 1 && arguments[0] == "--help" {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    match options(arguments.into_iter()).and_then(run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, Options, OsString, Result, options};

    fn parse(args: &[&str]) -> Result<Options> {
        options(args.iter().map(|value| OsString::from(*value)))
    }

    #[test]
    fn complete_file_and_repository_modes_are_explicit() -> Result<()> {
        assert_eq!(
            parse(&["ship", "--source", "x", "--check", "out"])?.kind,
            Kind::Ship
        );
        assert_eq!(
            parse(&[
                "station",
                "--repository",
                "--option-like-directory",
                "--output",
                "out"
            ])?
            .kind,
            Kind::Station
        );
        assert_eq!(
            parse(&[
                "all",
                "--ship-source",
                "a",
                "--station-source",
                "b",
                "--check",
                "out"
            ])?
            .kind,
            Kind::All
        );
        Ok(())
    }

    #[test]
    fn missing_duplicate_mixed_and_unknown_arguments_refuse() {
        for args in [
            vec![],
            vec!["SH029"],
            vec!["ship", "--source"],
            vec!["ship", "--source", "a"],
            vec!["station", "--ship-source", "a", "--output", "out"],
            vec![
                "all",
                "--repository",
                "r",
                "--ship-source",
                "a",
                "--station-source",
                "b",
                "--output",
                "out",
            ],
            vec!["ship", "--source", "a", "--source", "b", "--check", "out"],
            vec!["ship", "--source", "a", "--check", "out", "--output", "out"],
            vec![
                "ship",
                "--repository",
                "r",
                "--id",
                "SH029",
                "--check",
                "out",
            ],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
    }
}
