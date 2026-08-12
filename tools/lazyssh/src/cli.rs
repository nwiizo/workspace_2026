use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Eq, PartialEq)]
pub struct Cli {
    pub config: PathBuf,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Outcome {
    Run(Cli),
    Help,
    Version,
}

pub fn parse<I>(arguments: I, default_config: PathBuf) -> Result<Outcome, String>
where
    I: IntoIterator<Item = OsString>,
{
    let mut arguments = arguments.into_iter();
    let _program = arguments.next();
    let mut config = None;

    while let Some(argument) = arguments.next() {
        if argument == "-h" || argument == "--help" {
            return Ok(Outcome::Help);
        }
        if argument == "-V" || argument == "--version" {
            return Ok(Outcome::Version);
        }
        if argument == "-c" || argument == "--config" {
            if config.is_some() {
                return Err("--config may only be specified once".into());
            }
            config = Some(
                arguments
                    .next()
                    .filter(|value| !value.is_empty())
                    .map(PathBuf::from)
                    .ok_or_else(|| "--config requires a path".to_owned())?,
            );
            continue;
        }
        if let Some(value) = argument
            .to_str()
            .and_then(|value| value.strip_prefix("--config="))
        {
            if config.is_some() {
                return Err("--config may only be specified once".into());
            }
            if value.is_empty() {
                return Err("--config requires a path".into());
            }
            config = Some(PathBuf::from(value));
            continue;
        }

        return Err(format!("unknown argument: {}", argument.to_string_lossy()));
    }

    Ok(Outcome::Run(Cli {
        config: config.unwrap_or(default_config),
    }))
}
