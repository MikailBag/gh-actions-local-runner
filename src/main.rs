#[macro_use]
extern crate pest_derive;

mod hir;

use std::{path::PathBuf, process::exit};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    path: PathBuf,
}

fn main() {
    let opts: Opts = Opts::from_args();
    let workflow_path = opts.path.join(".github/main.workflow");
    if !workflow_path.exists() {
        eprintln!("error: {} does not exist", workflow_path.display());
        exit(1);
    }
    let workflow_data = std::fs::read_to_string(workflow_path).expect("failed read workflow");
    let defs = hir::parser::parse(&workflow_data);
    dbg!(&defs);
}
