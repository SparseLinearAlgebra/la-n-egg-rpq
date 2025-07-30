use std::{collections::HashMap, io, path::Path};

use egg::{Id, RecExpr};

use crate::{
    plan::Plan,
    query::{Pattern, Query, Vertex},
};

pub struct Graph {
    nvals: HashMap<String, usize>,
}

impl Graph {
    fn plan_aux(&self, expr: &mut RecExpr<Plan>, pattern: Pattern) -> Id {
        match pattern {
            Pattern::Uri(uri) => expr.add(Plan::Label(*self.nvals.get(&uri).expect("TBD"))),
            Pattern::Seq(lhs, rhs) => {
                let lhs = self.plan_aux(expr, *lhs);
                let rhs = self.plan_aux(expr, *rhs);
                expr.add(Plan::Seq([lhs, rhs]))
            }
            Pattern::Alt(lhs, rhs) => {
                let lhs = self.plan_aux(expr, *lhs);
                let rhs = self.plan_aux(expr, *rhs);
                expr.add(Plan::Alt([lhs, rhs]))
            }
            Pattern::Star(lhs) => {
                let lhs = self.plan_aux(expr, *lhs);
                expr.add(Plan::Star([lhs]))
            }
            Pattern::Plus(lhs) => {
                let lhs = self.plan_aux(expr, *lhs);
                let aux = expr.add(Plan::Star([lhs]));
                expr.add(Plan::Seq([lhs, aux]))
            }
            _ => todo!("TBD"),
        }
    }

    pub fn run(&self, query: Query) -> RecExpr<Plan> {
        let mut expr: RecExpr<Plan> = RecExpr::default();
        match query {
            Query {
                src: Vertex::Con(_v),
                pattern,
                dest: Vertex::Any,
            } => {
                // TODO: add src vertex knowledge.
                self.plan_aux(&mut expr, pattern)
            }
            Query {
                src: Vertex::Any,
                pattern,
                dest: Vertex::Con(_v),
            } => {
                // TODO: add dest vertex knowledge.
                self.plan_aux(&mut expr, pattern)
            }
            _ => todo!("TBD"),
        };
        expr
    }
}

pub fn load_dir(path: &Path) -> io::Result<Graph> {
    let dirs = std::fs::read_dir(path)?;

    let edges_file = path.join("edges.txt");
    let edges: HashMap<usize, String> = std::fs::read_to_string(edges_file)?
        .lines()
        .filter_map(|line| {
            let mut splits = line.split_whitespace();
            let edge = splits.next()?;
            let edge = edge[1..edge.len()].to_string();
            let num = splits.next()?.parse::<usize>().ok()?;
            Some((num, edge))
        })
        .collect();

    let nvals: HashMap<String, usize> = dirs
        .flatten()
        .map(|entry| entry.path())
        .filter_map(|entry| Some(entry.file_stem()?.to_str()?.to_string()))
        .filter_map(|entry| entry.parse::<usize>().ok())
        .filter_map(|entry| {
            let edge = edges.get(&entry)?;
            let edge_file = path.join(format!("{}.txt", entry));
            // TODO: read only first 3 lines :/.
            let edge_nvals = std::fs::read_to_string(edge_file)
                .ok()?
                .lines()
                .nth(2)?
                .split_whitespace()
                .nth(2)?
                .parse::<usize>()
                .ok()?;

            Some((edge.clone(), edge_nvals))
        })
        .collect();

    Ok(Graph { nvals })
}
