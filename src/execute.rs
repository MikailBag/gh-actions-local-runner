use crate::hir::{ActionId, Hir};
use std::{
    path::{Path, PathBuf},
    process::exit,
};

fn provide_docker_image(s: &str) -> String {
    if s.starts_with("docker://") {
        s.trim_start_matches("docker://").to_string()
    } else {
        eprintln!("only pre-built image references (docker://*) are supported currently");
        std::process::exit(1);
    }
}

pub struct GitInfo {
    commit: String,
}

pub struct ExecOptions {
    pub git: GitInfo,
    pub dir: PathBuf,
    pub dry: bool,
    pub home_dir: String,
    pub workspace_dir: String,
}

pub fn get_git_info(dir: &Path) -> GitInfo {
    let output = std::process::Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .stderr(std::process::Stdio::inherit())
        .current_dir(dir)
        .output()
        .expect("failed start git");
    if !output.status.success() {
        eprintln!("git didn't exit successfully");
        exit(1);
    }
    let commit = String::from_utf8(output.stdout).expect("strange git output").trim().to_string();
    GitInfo { commit }
}

pub fn execute(h: &Hir, act: ActionId, opts: &ExecOptions) {
    let act = h.action(act);
    println!("---executing action {}---", &act.name);
    let image = provide_docker_image(&act.uses);
    println!("Using docker image {}", &image);
    let mut cmdline = vec![];
    cmdline.push("run".to_string());
    cmdline.push("--rm".to_string());

    let mut env = act.env.clone();
    let mut add_var = |name: &str, value: &dyn AsRef<str>| {
        env.insert(name.to_string(), value.as_ref().to_string());
    };
    add_var("HOME", &"/github/home");
    add_var("GITHUB_WORKFLOW", &h.workflow().name);
    add_var("GITHUB_ACTION", &act.name);
    add_var("GITHUB_EVENT_NAME", &"push");
    add_var("GITHUB_EVENT_PATH", &"/github/workflow/event.json");
    add_var("GITHUB_WORKSPACE", &"/github/workspace");
    add_var("GITHUB_SHA", &opts.git.commit);

    add_var("GH_ACTIONS_RUNNER_LOCAL", &"1");

    for (key, val) in &env {
        cmdline.push("--env".to_string());
        cmdline.push(format!("{}={}", key, val));
    }
    cmdline.push("--workdir".to_string());
    cmdline.push("/github/workspace".to_string());

    cmdline.push("--volume".to_string());
    cmdline.push(format!("{}:/github/workspace", &opts.workspace_dir));

    cmdline.push("--volume".to_string());
    cmdline.push(format!("{}:/github/home", &opts.home_dir));

    let add_args;
    match &act.runs {
        Some(runs) => {
            let mut runs = runs.clone();
            if let Some(args) = &act.args {
                runs.extend(args.iter().cloned());
            }
            cmdline.push("--entrypoint".to_string());
            cmdline.push(runs[0].clone());
            add_args = runs[1..].to_vec();
        }
        None => {
            if let Some(args) = &act.args {
                add_args = args.clone();
            } else {
                add_args = vec![]
            }
        }
    }
    cmdline.push(image);
    cmdline.extend(add_args);
    println!("will run: {}", cmdline.join(" "));
    if opts.dry {
        return;
    }
    let st = match std::process::Command::new("docker").args(cmdline).status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed start docker: {}", e);
            std::process::exit(1);
        }
    };
    std::process::exit(st.code().unwrap_or(1));
}
