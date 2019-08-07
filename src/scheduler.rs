use crate::hir::{Hir, ActionId};

#[derive(Copy, Clone)]
enum Vis {
    Unvis,
    InProg(usize),
    Done,
}

struct Helper<'a> {
    vis: Vec<Vis>,
    order: Vec<usize>,
    g: &'a [Vec<usize>],
    stack: Vec<usize>,
    hir: &'a Hir,
}

impl<'a> Helper<'a> {
    fn new(h: &'a Hir, g: &'a [Vec<usize>]) -> Helper<'a> {
        let n = h.actions_cnt();

        Helper {
            g,
            vis: vec![Vis::Unvis; n],
            order: Vec::with_capacity(n),
            stack: Vec::with_capacity(n),
            hir: h,
        }
    }
    fn print_cycle(&self, cycle_top: usize) {
        let stack_size = self.stack.len();
        let cycle_len = stack_size - cycle_top;
        for i in 0..cycle_len {
            let u_pos = cycle_top + i;
            let v_pos = if i < cycle_len - 1 {
                cycle_top + i + 1
            } else {
                cycle_top
            };
            let u = ActionId::new(self.stack[u_pos]);
            let v = ActionId::new(self.stack[v_pos]);
            let u = self.hir.action(u);
            let v = self.hir.action(v);
            println!("{} -> {}", &u.name, &v.name);
        }
    }

    fn dfs(&mut self, v: usize) {
        self.vis[v] = Vis::InProg(self.stack.len());
        self.stack.push(v);
        for &w in &self.g[v] {
            match self.vis[w] {
                Vis::Done => {}
                Vis::InProg(j) => {
                    eprintln!("actions are cycled");
                    self.print_cycle(j);
                    std::process::exit(1);
                }
                Vis::Unvis => {
                    self.dfs(w);
                }
            }
        }
        self.order.push(v);
        self.vis[v] = Vis::Done;
        self.stack.pop();
    }

    fn run_all(&mut self) {
        for i in 0..self.g.len() {
            match self.vis[i] {
                Vis::Unvis => {
                    self.dfs(i)
                }
                Vis::InProg(_) => unreachable!(),
                Vis::Done => {}
            }
        }
    }
}

pub fn schedule(h: &Hir) -> Vec<ActionId> {
    let n = h.actions_cnt();

    let mut g = vec![vec![]; n];
    for i in 0..n {
        for &dep in &h.action(ActionId::new(i)).needs {
            g[i].push(dep.as_inner())
        }
    }
    let mut helper = Helper::new(h, &g);
    helper.run_all();

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(ActionId::new(helper.order[i]));
    }
    out
}