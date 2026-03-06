use std::{
    collections::HashMap,
    ffi::CString,
    fs::File,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
};

use egg::{Id, RecExpr};

use crate::{
    eval::{LAGraph_MMRead, LAGraph_RPQMatrix_reduce},
    grb,
    plan::{LabelMeta, Plan},
    query::{Pattern, Query, Vertex},
};

pub struct Graph {
    nvals: HashMap<String, usize>,
    pub size: usize,
    pub mat_files: HashMap<String, PathBuf>,
    pub mats: HashMap<String, grb::Matrix>,
    pub verts: HashMap<String, usize>,
    pub nvals_reduces: HashMap<String, (usize, usize)>,
}

impl Graph {
    fn ensure_label_loaded(&mut self, uri: &str) -> Result<(), String> {
        if self.mats.contains_key(uri) && self.nvals_reduces.contains_key(uri) {
            return Ok(());
        }

        let file = self
            .mat_files
            .get(uri)
            .cloned()
            .ok_or_else(|| format!("no matrix file for {}", uri))?;

        let mut mat = grb::Matrix(std::ptr::null_mut());

        unsafe {
            let c_file = CString::new(file.to_str().unwrap()).unwrap();
            let mode = CString::new("r").unwrap();

            let f = libc::fopen(c_file.as_ptr(), mode.as_ptr());
            if f.is_null() {
                return Err(format!("fopen failed for {}", file.display()));
            }

            let code = LAGraph_MMRead(&mut mat, f, std::ptr::null_mut());
            libc::fclose(f);

            if code != 0 {
                return Err(format!(
                    "unable to load matrix for {} in {} (error {})",
                    uri,
                    file.display(),
                    code
                ));
            }
        }

        let mut nnz_rows: usize = 0;
        let mut nnz_cols: usize = 0;

        unsafe {
            let code = LAGraph_RPQMatrix_reduce(&mut nnz_rows, mat, 0);
            if code != 0 {
                return Err(format!(
                    "unable to compute row reduce for {} in {} (error {})",
                    uri,
                    file.display(),
                    code
                ));
            }

            let code = LAGraph_RPQMatrix_reduce(&mut nnz_cols, mat, 1);
            if code != 0 {
                return Err(format!(
                    "unable to compute col reduce for {} in {} (error {})",
                    uri,
                    file.display(),
                    code
                ));
            }
        }

        self.mats.insert(uri.to_string(), mat);
        self.nvals_reduces
            .insert(uri.to_string(), (nnz_rows, nnz_cols));

        Ok(())
    }

    fn plan_aux(&mut self, expr: &mut RecExpr<Plan>, pattern: Pattern) -> Result<Id, String> {
        match pattern {
            Pattern::Uri(uri) => {
                self.ensure_label_loaded(&uri)?;

                let reduces = self
                    .nvals_reduces
                    .get(&uri)
                    .ok_or_else(|| format!("no such label: {}", uri))?;

                Ok(expr.add(Plan::Label(LabelMeta {
                    nvals: *self
                        .nvals
                        .get(&uri)
                        .ok_or_else(|| format!("no such label: {}", uri))?,
                    name: uri,
                    rreduce_nvals: reduces.0,
                    creduce_nvals: reduces.1,
                })))
            }
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

    pub fn run(&mut self, query: Query) -> Result<RecExpr<Plan>, String> {
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

    let mat_files: HashMap<String, PathBuf> = dirs
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

    let mut size: usize = 0;
    let nvals: HashMap<String, usize> = mat_files
        .iter()
        .filter_map(|(edge, file)| {
            let f = File::open(file).ok()?;
            let mut lines = BufReader::new(f).lines();

            lines.next()?.ok()?;
            lines.next()?.ok()?;
            let third_str = lines.next()?.ok()?;

            let mut parts = third_str.split_whitespace();
            let nrows = parts.next()?.parse::<usize>().ok()?;
            let ncols = parts.next()?.parse::<usize>().ok()?;
            let nnz = parts.next()?.parse::<usize>().ok()?;

            if nrows != ncols {
                panic!("matrix should be squared");
            }

            if size == 0 {
                size = nrows;
            } else if size != nrows {
                panic!(
                    "all matrices should have the same size, got {} and {}",
                    size, nrows
                );
            }

            Some((edge.clone(), nnz))
        })
        .collect();

    Ok(Graph {
        nvals,
        size,
        mat_files,
        mats: HashMap::new(),
        verts,
        nvals_reduces: HashMap::new(),
    })
}