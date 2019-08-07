pub mod parser;

use std::collections::HashMap;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct ActionId(usize);

#[derive(Debug, Clone)]
pub enum Event {
    Push,
}

#[derive(Debug, Clone)]
pub struct Workflow {
    pub name: String,
    pub on: Event,
    pub depends: Vec<ActionId>,
}

#[derive(Clone, Debug)]
pub struct Action {
    pub uses: String,
    pub runs: Vec<String>,
    pub needs: Vec<ActionId>,
    pub env: HashMap<String, String>,
    pub secrets: Vec<String>,
}

/// Top-level struct
pub struct Hir {
    workflow: Workflow,
    actions: Vec<Action>,
}

impl Hir {
    pub fn action(&self, id: ActionId) -> Action {
        self.actions[id.0].clone()
    }
}
