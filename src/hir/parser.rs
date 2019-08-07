use pest::{iterators::Pair, Parser, Span};
use std::{
    collections::{HashMap, HashSet}, process::exit,
};

#[derive(Parser)]
#[grammar = "hir/workflow.pest"]
struct WfParser;

#[derive(Debug, Clone)]
pub struct WorkflowDef {
    pub name: String,
    pub on: String,
    pub resolves: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ActionDef {
    pub name: String,
    pub uses: String,
    pub needs: Vec<String>,
    pub runs: Option<Vec<String>>,
    pub args: Option<Vec<String>>,
    pub env: HashMap<String, String>,
    pub secrets: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Def {
    Workflow(WorkflowDef),
    Action(ActionDef),
}

fn trim_str(s: &str) -> &str {
    let n = s.len();
    &s[1..n - 1]
}

fn fail_with_error(p: Span, err: impl std::string::ToString) -> ! {
    let msg = err.to_string();
    let err = pest::error::Error::<Rule>::new_from_span(
        pest::error::ErrorVariant::CustomError { message: msg },
        p,
    );
    eprintln!("error: {}", err);
    std::process::exit(1);
}

enum Value {
    String(String),
    Array(Vec<String>),
    Map(HashMap<String, String>),
}

impl Value {
    fn array_or_from_string(self, p: Span) -> Vec<String> {
        match self {
            Value::String(s) => vec![s],
            Value::Array(a) => a,
            Value::Map(_) => fail_with_error(p, "expected array or single string, got map"),
        }
    }

    fn array_or_split_string(self, p: Span) -> Vec<String> {
        match self {
            Value::String(s) => {
                s.split_ascii_whitespace().map(ToOwned::to_owned).collect()
            }
            Value::Array(arr) => arr,
            Value::Map(_) => fail_with_error(p, "expected array or string with space-separated items, got map"),
        }
    }

    fn ty(&self) -> &str {
        match self {
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Map(_) => "map",
        }
    }

    fn string(self, p: Span) -> String {
        match self {
            Value::String(s) => s,
            _ => fail_with_error(p, format!("expected string, got {}", self.ty()))
        }
    }

    fn array(self, p: Span) -> Vec<String> {
        match self {
            Value::Array(a) => a,
            _ => fail_with_error(p, format!("expected array, got {}", self.ty()))
        }
    }

    fn map(self, p: Span) -> HashMap<String, String> {
        match self {
            Value::Map(m) => m,
            _ => fail_with_error(p, format!("expected map, got {}", self.ty()))
        }
    }
}

struct KvPair {
    ident: String,
    val: Value,
}

fn parse_kvp(p: Pair<Rule>) -> KvPair {
    assert_eq!(p.as_rule(), Rule::kvp);
    let mut iter = p.into_inner();
    let ident = iter.next().unwrap();
    let value = iter.next().unwrap();
    let val = match value.as_rule() {
        Rule::string => Value::String(trim_str(value.as_str()).to_string()),
        Rule::array => {
            let mut out = vec![];
            for x in value.into_inner() {
                out.push(trim_str(x.as_str()).to_string());
            }
            Value::Array(out)
        }
        Rule::map => {
            let mut out = HashMap::new();
            let mut iter = value.into_inner();
            while let Some(ident) = iter.next() {
                assert_eq!(ident.as_rule(), Rule::ident);
                let val = iter.next().unwrap();
                out.insert(ident.as_str().to_string(), trim_str(val.as_str()).to_string());
            }
            Value::Map(out)
        }
        _ => {
            dbg!(value);
            unreachable!()
        }
    };
    KvPair {
        ident: ident.as_str().to_string(),
        val,
    }
}

fn parse_def_body(p: Pair<Rule>, required: &[&str]) -> (String, HashMap<String, Value>) {
    let mut out = HashMap::new();
    let all_span = p.as_span();
    let mut iter = p.into_inner();
    let name = iter.next().unwrap().as_str();
    for item in iter {
        let item_span = item.as_span();
        let KvPair { ident, val } = parse_kvp(item);
        if out.insert(ident.clone(), val).is_some() {
            fail_with_error(item_span, format!("key {} redefined", ident));
        }
    }
    for &req in required {
        if !out.contains_key(req) {
            fail_with_error(all_span, format!("key {} required but not set", req));
        }
    }
    (trim_str(name).to_string(), out)
}

fn check_kvps_empty(p: Span, k: &HashMap<String, Value>) {
    if let Some(some_key) = k.keys().next() {
        fail_with_error(p, format!("unexpected key {}", some_key))
    }
}

fn parse_workflow_def(p: Pair<Rule>) -> WorkflowDef {
    assert_eq!(p.as_rule(), Rule::workflow_def);
    let span = p.as_span();
    let (name, mut kvps) = parse_def_body(p, &["on", "resolves"]);
    let out = WorkflowDef {
        name,

        on: kvps.remove("on").unwrap().string(span.clone()),
        resolves: kvps.remove("resolves").unwrap().array_or_from_string(span.clone()),
    };
    check_kvps_empty(span, &kvps);
    out
}

fn parse_action_def(p: Pair<Rule>) -> ActionDef {
    assert_eq!(p.as_rule(), Rule::action_def);
    let span = p.as_span();
    let (name, mut kvps) = parse_def_body(p, &["uses"]);
    let out = ActionDef {
        name,

        uses: kvps.remove("uses").unwrap().string(span.clone()),
        needs: kvps.remove("needs").map(|x| x.array_or_from_string(span.clone())).unwrap_or_default(),
        runs: kvps.remove("runs").map(|x| x.array_or_split_string(span.clone())),
        args: kvps.remove("args").map(|x| x.array_or_split_string(span.clone())),
        env: kvps.remove("env").map(|x| x.map(span.clone())).unwrap_or_default(),
        secrets: kvps.remove("secrets").map(|x| x.array(span.clone())).unwrap_or_default(),
    };

    check_kvps_empty(span, &kvps);
    out
}

fn parse_def(p: Pair<Rule>) -> Def {
    match p.as_rule() {
        Rule::workflow_def => Def::Workflow(dbg!(parse_workflow_def(p))),
        Rule::action_def => Def::Action(dbg!(parse_action_def(p))),
        _ => unreachable!(),
    }
}

pub fn parse(s: &str) -> Vec<Def> {
    let mut tree = match WfParser::parse(Rule::all, s) {
        Ok(x) => x,
        Err(err) => {
            eprintln!("syntax error: {}", err);
            std::process::exit(1);
        }
    };
    let mut res = Vec::new();
    let defs = tree.next().unwrap();
    for x in defs.into_inner() {
        if x.as_rule() == Rule::EOI {
            break;
        }
        let def = parse_def(x);
        res.push(def);
    }
    res
}
