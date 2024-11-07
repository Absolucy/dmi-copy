// SPDX-License-Identifier: MPL-2.0
#![warn(
	clippy::correctness,
	clippy::suspicious,
	clippy::complexity,
	clippy::perf,
	clippy::style
)]

mod args;

use color_eyre::eyre::{Result, WrapErr};
use dmi::icon::Icon;
use std::{
	fs::File,
	io::{BufReader, BufWriter},
	path::Path,
};

fn main() -> Result<()> {
	color_eyre::install()?;
	let args = args::DmiCopyArgs::parse().wrap_err("failed to parse arguments")?;
	let from = load_dmi(&args.from)
		.wrap_err_with(|| format!("failed to read input file {}", args.from.display()))?;
	let mut to = load_dmi(&args.to)
		.wrap_err_with(|| format!("failed to read output file {}", args.from.display()))?;

	let states_to_insert = from
		.states
		.iter()
		.filter(|state| args.icon_states.contains(&state.name))
		.cloned()
		.filter_map(|new_state| {
			let name = new_state.name.as_str();
			match to
				.states
				.iter_mut()
				.find(|existing_state| existing_state.name == name)
			{
				Some(existing_state) => {
					if *existing_state == new_state {
						println!("State '{name}' identical in both files");
					} else {
						println!("State '{name}' replaced");
						*existing_state = new_state;
					}
					None
				}
				None => Some(new_state),
			}
		})
		.collect::<Vec<_>>();

	to.states.reserve(states_to_insert.len());
	for new_state in states_to_insert {
		println!("State '{}' added", new_state.name);
		to.states.push(new_state);
	}

	save_dmi(to, &args.to)
		.wrap_err_with(|| format!("failed to save dmi to {}", args.to.display()))?;

	println!("done!");

	Ok(())
}

fn load_dmi(path: &Path) -> Result<Icon> {
	let file = File::open(path)
		.map(BufReader::new)
		.wrap_err("failed to open file for reading")?;
	Icon::load(file).wrap_err("failed to load dmi")
}

fn save_dmi(dmi: Icon, path: &Path) -> Result<()> {
	// For the sake of user safety, we do an "atomic write" by writing to a
	// tempfile, and then copying said tempfile to the target path.
	let mut file = tempfile::Builder::new()
		.suffix(".dmi")
		.tempfile()
		.map(BufWriter::new)
		.wrap_err("failed to create temporary output file")?;
	dmi.save(&mut file).wrap_err("failed to save dmi")?;
	let file = file
		.into_inner()
		.wrap_err("failed to finish writing buffer to file")?;
	std::fs::copy(file.path(), path).wrap_err("failed to copy temp file to target")?;
	Ok(())
}
