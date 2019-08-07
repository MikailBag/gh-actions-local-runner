#[macro_use]
extern crate pest_derive;

mod execute;
mod hir;
mod scheduler;

use std::{path::{PathBuf, Path}, process::exit};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    path: PathBuf,
    #[structopt(short = "d", long = "dry")]
    dry_run: bool,
}

fn deep_copy_with_ignore(src: &Path, dest: &Path) {
    let walker = ignore::Walk::new(src);
    for item in walker {
        let item = item.expect("failed process item");
        let is_file = match item.file_type() {
            Some(s) => s.is_file(),
            None => false,
        };
        if !is_file {
            continue;
        }
        let path = item.path();
        let rel_path = path.strip_prefix(src).expect("failed strip src path");
        let dest_path = dest.join(rel_path);
    }
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
    let hir = hir::build(defs);
    let actions_in_order = scheduler::schedule(&hir);

    let workspace_dir = tempfile::tempdir().expect("failed allocate workspace dir");
    let home_dir = tempfile::tempdir().expect("failed allocate home dir");
    let workspace_dir = workspace_dir
        .into_path()
        .to_str()
        .expect("system temp dir path is not utf8")
        .to_string();
    let home_dir = home_dir
        .into_path()
        .to_str()
        .expect("system temp dir path is not utf8")
        .to_string();

    deep_copy_with_ignore(&opts.path, Path::new(&workspace_dir));

    let ex_opts = execute::ExecOptions {
        dry: opts.dry_run,
        dir: opts.path.clone(),
        git: execute::get_git_info(&opts.path),
        home_dir,
        workspace_dir,
    };

    for act in actions_in_order {
        execute::execute(&hir, act, &ex_opts);
    }
}
