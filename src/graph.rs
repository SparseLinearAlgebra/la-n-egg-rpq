use std::{
    collections::HashMap,
    ffi::CString,
    io,
    path::{Path, PathBuf},
};

use egg::{Id, RecExpr};
use libc::c_ulonglong;

use crate::{
    eval::{LAGraph_MMRead, LAGraph_RPQMatrix_reduce},
    grb,
    plan::{LabelMeta, Plan},
    query::{Pattern, Query, Vertex},
};

pub struct Graph {
    nvals: HashMap<String, usize>,
    pub mats: HashMap<String, grb::Matrix>,
    pub verts: HashMap<String, usize>,
    pub nvals_reduces: HashMap<String, (libc::c_ulonglong, libc::c_ulonglong)>,
}

impl Graph {
    fn plan_aux(&self, expr: &mut RecExpr<Plan>, pattern: Pattern) -> Result<Id, String> {
        match pattern {
            Pattern::Uri(uri) => Ok(expr.add(Plan::Label(LabelMeta {
                nvals: *self
                    .nvals
                    .get(&uri)
                    .ok_or(format!("no such label: {}", uri))?,
                name: uri,
                rreduce_nvals: *self
                    .nvals_reduces
                    .get(&uri)
                    .0
                    .ok_or(format!("no such label: {}", uri))?,
                creduce_nvals: *self
                    .nvals_reduces
                    .get(&uri)
                    .1
                    .ok_or(format!("no such label: {}", uri))?,
            }))),
            Pattern::Seq(lhs, rhs) => {
                let lhs = self.plan_aux(expr, *lhs)?;
                let rhs = self.plan_aux(expr, *rhs)?;
                Ok(expr.add(Plan::Seq([lhs, rhs])))
            }
            Pattern::Alt(lhs, rhs) => {
                let lhs = self.plan_aux(expr, *lhs)?;
                let rhs = self.plan_aux(expr, *rhs)?;
                Ok(expr.add(Plan::Alt([lhs, rhs])))
            }
            Pattern::Star(lhs) => {
                let lhs = self.plan_aux(expr, *lhs)?;
                Ok(expr.add(Plan::Star([lhs])))
            }
            Pattern::Plus(lhs) => {
                let lhs = self.plan_aux(expr, *lhs)?;
                let aux = expr.add(Plan::Star([lhs]));
                Ok(expr.add(Plan::Seq([lhs, aux])))
            }
            Pattern::Opt(_lhs) => {
                todo!("opt (?) queries are not supported yet")
            }
        }
    }

    pub fn run(&self, query: Query) -> Result<RecExpr<Plan>, String> {
        let mut expr: RecExpr<Plan> = RecExpr::default();
        match query {
            Query {
                src: Vertex::Any,
                pattern,
                dest: Vertex::Any,
            } => self.plan_aux(&mut expr, pattern)?,
            Query {
                src: Vertex::Con(name),
                pattern,
                dest: Vertex::Any,
            } => {
                let lhs = expr.add(Plan::Label(LabelMeta {
                    name,
                    nvals: 1,
                    rreduce_nvals: 1,
                    creduce_nvals: 1,
                }));
                let rhs = self.plan_aux(&mut expr, pattern)?;
                expr.add(Plan::Seq([lhs, rhs]))
            }
            Query {
                src: Vertex::Any,
                pattern,
                dest: Vertex::Con(name),
            } => {
                let lhs = self.plan_aux(&mut expr, pattern)?;
                let rhs = expr.add(Plan::Label(LabelMeta {
                    name,
                    nvals: 1,
                    rreduce_nvals: 1,
                    creduce_nvals: 1,
                }));
                expr.add(Plan::Seq([lhs, rhs]))
            }
            Query {
                src: Vertex::Con(_v1),
                pattern: _,
                dest: Vertex::Con(_v2),
            } => {
                todo!("con to con queries are not supported yet")
            }
        };
        Ok(expr)
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
            let edge = edge[1..edge.len() - 1].to_string();
            let num = splits.next()?.parse::<usize>().ok()?;
            Some((num, edge))
        })
        .collect();

    let verts_file = path.join("vertices.txt");
    let verts: HashMap<String, usize> = std::fs::read_to_string(verts_file)?
        .lines()
        .filter_map(|line| {
            let mut splits = line.split_whitespace();
            let vert = splits.next()?;
            let vert = vert[1..vert.len() - 1].to_string();
            let num = splits.next()?.parse::<usize>().ok()?;
            Some((vert, num))
        })
        .collect();

    let mat_files: Vec<(String, PathBuf)> = dirs
        .flatten()
        .map(|entry| entry.path())
        .filter_map(|entry| Some(entry.file_stem()?.to_str()?.to_string()))
        .filter_map(|entry| entry.parse::<usize>().ok())
        .filter_map(|entry| {
            Some((
                edges.get(&entry)?.clone(),
                path.join(format!("{}.txt", entry)),
            ))
        })
        .collect();

    let nvals: HashMap<String, usize> = mat_files
        .iter()
        .filter_map(|(edge, file)| {
            // TODO: read only first 3 lines :/.
            let edge_nvals = std::fs::read_to_string(file)
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

    let mats: HashMap<String, grb::Matrix> = mat_files
        .iter()
        .map(|(edge, file)| {
            let mut mat = grb::Matrix(std::ptr::null_mut());
            unsafe {
                let c_file = CString::new(file.to_str().unwrap()).unwrap();
                let mode = CString::new("r").unwrap();
                let f = libc::fopen(c_file.as_ptr(), mode.as_ptr());
                let code = LAGraph_MMRead(&mut mat as *mut grb::Matrix, f, std::ptr::null_mut());
                assert_eq!(
                    code,
                    0,
                    "unable to load matrix for {} in {} (error {})",
                    edge,
                    file.display(),
                    code
                );
            };
            (edge.clone(), mat)
        })
        .collect();

    let nvals_reduces: HashMap<String, (usize, usize)> = mats
        .iter()
        .map(|(edge, mat): (String, &mut grb::Matrix)| {
            let mut nnz_rows = 0;
            let mut nnz_cols = 0;
            unsafe {
                let code = LAGraph_RPQMatrix_reduce(&mut nnz_rows, mat, reduce_type);
                Ok(code);
                let code = LAGraph_RPQMatrix_reduce(&mut nnz_cols, mat, reduce_type);
                Ok(code);
            };
            (edge.clone(), (nnz_rows, nnz_cols))
        })
        .collect();

    Ok(Graph {
        nvals,
        mats,
        verts,
        nvals_reduces,
    })
}
