mod lint;
pub mod parser;

use std::collections::HashMap;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct ActionId(usize);

impl ActionId {
    pub fn new(p: usize) -> Self {
        ActionId(p)
    }

    pub fn as_inner(&self) -> usize {
        self.0
    }
}

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
    pub name: String,
    pub uses: String,
    pub runs: Option<Vec<String>>,
    pub args: Option<Vec<String>>,
    pub needs: Vec<ActionId>,
    pub env: HashMap<String, String>,
    pub secrets: Vec<String>,
}

/// Top-level struct
#[derive(Clone, Debug)]
pub struct Hir {
    workflow: Workflow,
    actions: Vec<Action>,
}

impl Hir {
    pub fn action(&self, id: ActionId) -> &Action {
        &self.actions[id.0]
    }

    pub fn workflow(&self) -> &Workflow {
        &self.workflow
    }

    pub fn actions(&self) -> impl Iterator<Item=&Action> {
        self.actions.iter()
    }

    pub fn actions_cnt(&self) -> usize {
        self.actions.len()
    }
}

mod builder {
    use super::{parser::Def, Action, ActionId, Event, Hir, Workflow};
    use std::{collections::HashMap, process::exit};

    #[derive(Default)]
    struct ActionIdMapper {
        map: HashMap<String, ActionId>,
    }

    impl ActionIdMapper {
        fn new() -> ActionIdMapper {
            Default::default()
        }

        fn get(&self, s: &str) -> Option<ActionId> {
            self.map.get(s).copied()
        }

        fn feed(&mut self, s: &str) -> (bool, ActionId) {
            let new_id = ActionId(self.map.len());
            match self.get(s) {
                Some(id) => (false, id),
                None => {
                    self.map.insert(s.to_string(), new_id);
                    (true, new_id)
                }
            }
        }
    }

    fn generate_hir(defs: &[Def]) -> Hir {
        let mut actions = vec![];
        let mut workflow = None;
        let mut action_id_mapper = ActionIdMapper::new();
        for def in defs {
            if let Def::Action(def) = def {
                if !action_id_mapper.feed(&def.name).0 {
                    eprintln!("action with name {} already defined", def.name);
                    exit(1);
                }
            }
        }
        for def in defs {
            match def {
                Def::Workflow(def) => {
                    if workflow.is_some() {
                        eprintln!("multiple workflows are not supported");
                        exit(1);
                    }
                    let on = match def.on.as_str() {
                        "push" => Event::Push,
                        _ => {
                            eprintln!("unknown workflow trigger: {}", &def.on);
                            exit(1);
                        }
                    };
                    let mut depends = vec![];
                    for dep in &def.resolves {
                        let dep_id = action_id_mapper.get(dep).unwrap_or_else(|| {
                            eprintln!("workflow refers to unknown action {}", dep);
                            exit(1);
                        });
                        depends.push(dep_id);
                    }
                    workflow = Some(Workflow {
                        name: def.name.clone(),
                        on,
                        depends,
                    });
                }
                Def::Action(def) => {
                    let mut depends = vec![];
                    for dep in &def.needs {
                        let dep_id = action_id_mapper.get(dep).unwrap_or_else(|| {
                            eprintln!("action {} refers to unknown action {}", def.name, dep);
                            exit(1);
                        });
                        depends.push(dep_id);
                    }
                    actions.push(Action {
                        name: def.name.clone(),
                        uses: def.uses.clone(),
                        runs: def.runs.clone(),
                        args: def.args.clone(),
                        needs: depends,
                        env: def.env.clone(),
                        secrets: def.secrets.clone(),
                    })
                }
            }
        }
        let workflow = workflow.unwrap_or_else(|| {
            eprintln!("no workflows found");
            exit(1);
        });

        Hir { workflow, actions }
    }

    pub fn build(defs: Vec<Def>) -> Hir {
        generate_hir(&defs)
    }
}

pub fn build(defs: Vec<parser::Def>) -> Hir {
    let h = builder::build(defs);
    lint::lint(&h);
    h
}
