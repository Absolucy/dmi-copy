// SPDX-License-Identifier: MPL-2.0
use clap::{arg, command, value_parser, ArgAction, CommandFactory, Parser};
use clap_complete::{Generator, Shell};
use color_eyre::eyre::{eyre, Result};
use std::path::PathBuf;

/// Arguments for copying icon states between DMI files
#[derive(Debug)]
pub struct DmiCopyArgs {
	/// The original .dmi file to read the target icon states from
	pub from: PathBuf,
	/// The target .dmi file to copy the icon states into
	pub to: PathBuf,
	/// A list of the icon states to copy
	pub icon_states: Vec<String>,
}

/// Represents all possible ways to provide arguments
#[derive(Debug, Parser)]
#[command(
	about = "Copy icon states between DMI files",
	after_help = "EXAMPLES:\n    Natural syntax:\n        dmi-copy state1 state2 state3 from \
	              original.dmi to target.dmi\n\n    Traditional syntax:\n        dmi-copy --from \
	              original.dmi --to target.dmi --state state1,state2,state3\n        dmi-copy \
	              --from original.dmi --to target.dmi --state state1 --state state2",
	help_template = "{about}\n\nUSAGE:\n    Natural syntax:  {name} <STATES>... from <FROM> to \
	                 <TO>\n    Flag syntax:    {name} --from <FROM> --to <TO> --state \
	                 <STATES>...\n\nOPTIONS:\n{options}\n\n{after-help}"
)]
struct CliArgs {
	/// Non-flag arguments for natural syntax
	#[arg(
        value_parser = value_parser!(String),
        required = false,
        conflicts_with_all = &["from_flag", "to_flag", "state_flag"],
        hide = true
    )]
	natural_args: Vec<String>,

	/// Source DMI file (traditional syntax)
	#[arg(
        long = "from",
        value_name = "FILE",
        value_parser = value_parser!(PathBuf),
        requires_all = &["to_flag", "state_flag"],
        id = "from_flag",
        help = "The source .dmi file to copy states from"
    )]
	from: Option<PathBuf>,

	/// Target DMI file (traditional syntax)
	#[arg(
        long = "to",
        value_name = "FILE",
        value_parser = value_parser!(PathBuf),
        requires_all = &["from_flag", "state_flag"],
        id = "to_flag",
        help = "The target .dmi file to copy states into"
    )]
	to: Option<PathBuf>,

	/// Icon states to copy (traditional syntax)
	#[arg(
        long = "state",
        alias = "states",
        value_name = "STATE",
        value_parser = parse_state_arg,
        action = ArgAction::Append,
        requires_all = &["from_flag", "to_flag"],
        id = "state_flag",
        help = "Icon states to copy (can be comma-separated)"
    )]
	states: Option<Vec<Vec<String>>>,

	/// Generate shell completion script
	#[arg(
        long = "generate-completion",
        value_name = "SHELL",
        value_parser = value_parser!(Shell),
        help = "Generate completion script for specified shell"
    )]
	generate_completion: Option<Shell>,
}

/// Parse a comma-separated state argument into individual states
fn parse_state_arg(arg: &str) -> Result<Vec<String>, String> {
	Ok(arg
		.split(',')
		.map(|s| s.trim().to_string())
		.filter(|s| !s.is_empty())
		.collect())
}

impl DmiCopyArgs {
	/// Parse command line arguments into DmiCopyArgs
	pub fn parse() -> Result<Self> {
		match CliArgs::try_parse() {
			Ok(cli) => {
				// Handle completion generation if requested
				if let Some(shell) = cli.generate_completion {
					print_completions(shell, &mut CliArgs::command());
					std::process::exit(0);
				}

				if !cli.natural_args.is_empty() {
					// Handle natural syntax
					Self::parse_natural_syntax(&cli.natural_args)
				} else {
					// Handle traditional flag syntax
					if let (Some(from), Some(to), Some(states)) = (cli.from, cli.to, cli.states) {
						Ok(DmiCopyArgs {
							from,
							to,
							icon_states: states.into_iter().flatten().collect(),
						})
					} else {
						// Show help if no arguments are provided
						CliArgs::command().print_help().unwrap();
						std::process::exit(0);
					}
				}
			}
			Err(err) => {
				err.print().unwrap();
				std::process::exit(1);
			}
		}
	}

	/// Parse the natural command syntax
	fn parse_natural_syntax(args: &[String]) -> Result<Self> {
		let mut icon_states = Vec::new();
		let mut from = None;
		let mut to = None;
		let mut current_mode = ParseMode::States;

		for arg in args {
			match arg.as_str() {
				"from" => {
					if !icon_states.is_empty() {
						current_mode = ParseMode::From;
					} else {
						return Err(eyre!("No icon states specified before 'from'"));
					}
				}
				"to" => {
					if from.is_some() {
						current_mode = ParseMode::To;
					} else {
						return Err(eyre!("Source file not specified before 'to'"));
					}
				}
				value => match current_mode {
					ParseMode::States => icon_states.push(value.to_string()),
					ParseMode::From => {
						from = Some(PathBuf::from(value));
						current_mode = ParseMode::WaitingTo;
					}
					ParseMode::To => {
						to = Some(PathBuf::from(value));
						current_mode = ParseMode::Done;
					}
					ParseMode::WaitingTo => {
						return Err(eyre!("Expected 'to' keyword"));
					}
					ParseMode::Done => {
						return Err(eyre!("Unexpected additional arguments"));
					}
				},
			}
		}

		match (from, to) {
			(Some(from), Some(to)) => Ok(DmiCopyArgs {
				from,
				to,
				icon_states,
			}),
			(Some(_), None) => Err(eyre!("Missing destination file")),
			(None, Some(_)) => Err(eyre!("Missing source file")),
			(None, None) => Err(eyre!("Missing both source and destination file")),
		}
	}
}

fn print_completions<G: Generator>(gen: G, cmd: &mut clap::Command) {
	clap_complete::generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
}

#[derive(Debug)]
enum ParseMode {
	States,
	From,
	WaitingTo,
	To,
	Done,
}

#[cfg(test)]
mod tests {
	use super::*;
	use color_eyre::eyre::{eyre, Result, WrapErr};

	fn parse_args(args: &[&str]) -> Result<DmiCopyArgs> {
		// Prepend the binary name as clap expects it
		let args = std::iter::once("dmi-copy").chain(args.iter().copied());

		let cli = CliArgs::try_parse_from(args).wrap_err("failed to parse cil args")?;

		if !cli.natural_args.is_empty() {
			DmiCopyArgs::parse_natural_syntax(&cli.natural_args)
		} else if let (Some(from), Some(to), Some(states)) = (cli.from, cli.to, cli.states) {
			Ok(DmiCopyArgs {
				from,
				to,
				icon_states: states.into_iter().flatten().collect(),
			})
		} else {
			Err(eyre!("Missing required arguments"))
		}
	}

	#[test]
	fn test_natural_syntax() {
		let result = parse_args(&[
			"state1",
			"state2",
			"from",
			"original.dmi",
			"to",
			"target.dmi",
		]);
		assert!(result.is_ok());
		let args = result.unwrap();
		assert_eq!(args.icon_states, vec!["state1", "state2"]);
		assert_eq!(args.from, PathBuf::from("original.dmi"));
		assert_eq!(args.to, PathBuf::from("target.dmi"));
	}

	#[test]
	fn test_traditional_syntax() {
		let result = parse_args(&[
			"--from",
			"original.dmi",
			"--to",
			"target.dmi",
			"--state",
			"state1,state2",
		]);
		assert!(result.is_ok());
		let args = result.unwrap();
		assert_eq!(args.icon_states, vec!["state1", "state2"]);
		assert_eq!(args.from, PathBuf::from("original.dmi"));
		assert_eq!(args.to, PathBuf::from("target.dmi"));
	}

	#[test]
	fn test_traditional_syntax_multiple_flags() {
		let result = parse_args(&[
			"--from",
			"original.dmi",
			"--to",
			"target.dmi",
			"--state",
			"state1",
			"--state",
			"state2,state3",
		]);
		assert!(result.is_ok());
		let args = result.unwrap();
		assert_eq!(args.icon_states, vec!["state1", "state2", "state3"]);
		assert_eq!(args.from, PathBuf::from("original.dmi"));
		assert_eq!(args.to, PathBuf::from("target.dmi"));
	}

	#[test]
	fn test_invalid_natural_syntax() {
		// Missing 'from' keyword
		assert!(parse_args(&["state1", "original.dmi", "to", "target.dmi"]).is_err());

		// Missing 'to' keyword
		assert!(parse_args(&["state1", "from", "original.dmi", "target.dmi"]).is_err());

		// No states specified
		assert!(parse_args(&["from", "original.dmi", "to", "target.dmi"]).is_err());
	}

	#[test]
	fn test_invalid_traditional_syntax() {
		// Missing --from
		assert!(parse_args(&["--to", "target.dmi", "--state", "state1"]).is_err());

		// Missing --state
		assert!(parse_args(&["--from", "original.dmi", "--to", "target.dmi"]).is_err());

		// Missing --to
		assert!(parse_args(&["--from", "original.dmi", "--state", "state1"]).is_err());
	}

	#[test]
	fn test_traditional_syntax_empty_states() {
		let result = parse_args(&[
			"--from",
			"original.dmi",
			"--to",
			"target.dmi",
			"--state",
			"state1,,state2",
		]);
		assert!(result.is_ok());
		let args = result.unwrap();
		assert_eq!(args.icon_states, vec!["state1", "state2"]);
	}
}
