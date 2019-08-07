use super::Hir;
use std::{cmp::max, process::exit};

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
enum LintOutcome {
    Allow,
    Error,
}

impl LintOutcome {
    fn max_assign(&mut self, other: LintOutcome) {
        *self = max(*self, other);
    }
}

/// Lints env vars with name, starting with "GITHUB_"
fn lint_env_github(h: &Hir) -> LintOutcome {
    let mut outcome = LintOutcome::Allow;
    for act in h.actions() {
        for var_name in act.env.keys() {
            if var_name.starts_with("GITHUB_") {
                outcome.max_assign(LintOutcome::Error);
                eprint!("Error: in action '{}': in env var '{}':", &act.name, var_name);
                eprintln!("var name starts with reserved prefix GITHUB_");
            }
        }
    }
    outcome
}

/// Lints overflowing secrets limit
fn lint_secrets_limit(h: &Hir) -> LintOutcome {
    let mut secrets = std::collections::HashSet::new();
    for act in h.actions() {
        for secret in &act.secrets {
            secrets.insert(secret);
        }
    }
    const GITHUB_SECRET_COUNT_LIMIT: usize = 100;
    if secrets.len() > GITHUB_SECRET_COUNT_LIMIT {
        eprintln!("Error: you are using {} secrets, which exceeds GitHub limit of {}", secrets.len(), GITHUB_SECRET_COUNT_LIMIT);
        LintOutcome::Error
    } else {
        LintOutcome::Allow
    }
}

pub fn lint(h: &Hir) {
    let mut outcome = LintOutcome::Allow;
    outcome.max_assign(lint_env_github(h));
    outcome.max_assign(lint_secrets_limit(h));
    if outcome == LintOutcome::Error {
        exit(1);
    }
}
