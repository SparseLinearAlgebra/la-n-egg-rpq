use std::{cmp::Ordering, collections::HashMap, fmt::Display, str::FromStr};

use egg::*;
use libc::timegm;

use crate::{
    eval::{LAGraph_RPQMatrix_Alt, LAGraph_RPQMatrix_ExtractRandom, LAGraph_RPQMatrix_Seq},
    graph::Graph,
    grb,
};

#[derive(Clone, Hash, Ord, Eq, PartialEq, PartialOrd, Debug)]
pub struct LabelMeta {
    pub name: String,
    pub nvals: usize,
    pub rreduce_nvals: usize,
    pub creduce_nvals: usize,
}

impl FromStr for LabelMeta {
    type Err = <usize as FromStr>::Err;
    // This is needed for the builtin egg parser. Only used in tests.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(LabelMeta {
            name: "-".to_string(),
            nvals: s.parse()?,
            rreduce_nvals: s.parse()?,
            creduce_nvals: s.parse()?,
        })
    }
}

impl Display for LabelMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.name, self.nvals)
    }
}

define_language! {
pub enum Plan {
    Label(LabelMeta),
    "/" = Seq([egg::Id; 2]),
    "|" = Alt([egg::Id; 2]),
    "*" = Star([egg::Id; 1]),
    "l*" = LStar([egg::Id; 2]),
    "*r" = RStar([egg::Id; 2]),
} }

pub fn make_rules() -> Vec<egg::Rewrite<Plan, ()>> {
    vec![
        rewrite!("assoc-sec-1"; "(/ ?a (/ ?b ?c))" => "(/ (/ ?a ?b) ?c)"),
        rewrite!("assoc-sec-2"; "(/ (/ ?a ?b) ?c)" => "(/ ?a (/ ?b ?c))"),
        rewrite!("commute-alt"; "(| ?a ?b)" => "(| ?b ?a)"),
        rewrite!("assoc-alt"; "(| ?a (| ?b ?c))" => "(| (| ?a ?b) ?c)"),
        rewrite!("distribute-1"; "(/ ?a (| ?b ?c))" => "(| (/ ?a ?b) (/ ?a ?c))"),
        rewrite!("distribute-2"; "(/ (| ?a ?b) ?c)" => "(| (/ ?a ?c) (/ ?b ?c))"),
        rewrite!("distribute-3"; "(| (/ ?a ?b) (/ ?a ?c))" => "(/ ?a (| ?b ?c))"),
        rewrite!("distribute-4"; "(| (/ ?a ?c) (/ ?b ?c))" => "(/ (| ?a ?b) ?c)"),
        rewrite!("build-lstar"; "(/ (* ?a) ?b)" => "(l* ?a ?b)"),
        rewrite!("build-rstar"; "(/ ?a (* ?b))" => "(*r ?a ?b)"),
    ]
}

pub fn make_stupid_rules() -> Vec<egg::Rewrite<Plan, ()>> {
    vec![
        rewrite!("assoc-sec-1"; "(/ ?a (/ ?b ?c))" => "(/ (/ ?a ?b) ?c)"),
        rewrite!("assoc-sec-2"; "(/ (/ ?a ?b) ?c)" => "(/ ?a (/ ?b ?c))"),
        rewrite!("commute-alt"; "(| ?a ?b)" => "(| ?b ?a)"),
        rewrite!("assoc-alt"; "(| ?a (| ?b ?c))" => "(| (| ?a ?b) ?c)"),
    ]
}

pub struct RandomCostFn;
impl CostFunction<Plan> for RandomCostFn {
    type Cost = f64;
    fn cost<C>(&mut self, _enode: &Plan, _costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        rand::random()
    }
}

pub struct NnzCostFn;
impl CostFunction<Plan> for NnzCostFn {
    type Cost = f64;
    fn cost<C>(&mut self, enode: &Plan, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        match enode {
            Plan::Label(meta) => meta.nvals as f64,
            Plan::Seq(args) => costs(args[0]).min(costs(args[1])).powf(1.1),
            Plan::Alt(args) => costs(args[0]).min(costs(args[1])).powf(1.1),
            Plan::Star(args) => costs(args[0]).powi(2),
            Plan::LStar(args) => costs(args[0]) * costs(args[1]),
            Plan::RStar(args) => costs(args[0]) * costs(args[1]),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CardCost {
    pub score: f64,
    pub nnz: f64,
    pub nnz_r: f64,
    pub nnz_c: f64,
}

impl Eq for CardCost {}

impl PartialOrd for CardCost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CardCost {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.score.total_cmp(&other.score) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.nnz.total_cmp(&other.nnz) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.nnz_r.total_cmp(&other.nnz_r) {
            Ordering::Equal => {}
            ord => return ord,
        }
        self.nnz_c.total_cmp(&other.nnz_c)
    }
}

pub struct CardinalityCostFn;
impl CostFunction<Plan> for CardinalityCostFn {
    type Cost = CardCost;

    fn cost<C>(&mut self, enode: &Plan, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        match enode {
            Plan::Label(meta) => {
                let nnz_r = meta.rreduce_nvals as f64;
                let nnz_c = meta.creduce_nvals as f64;
                CardCost {
                    score: 0.0,
                    nnz: meta.nvals as f64,
                    nnz_r,
                    nnz_c,
                }
            }

            Plan::Seq([a, b]) => {
                // C = A x B
                let ca = costs(*a);
                let cb = costs(*b);

                // calculate score of C
                let denom = ca.nnz_r.max(cb.nnz_c).max(1.0);
                let op_cost = (ca.nnz * cb.nnz) / denom;

                let score = ca.score + cb.score + op_cost;

                // estimate nonzeros in C matrix reduced by rows and columns
                CardCost {
                    score: score,
                    nnz: 0.0,   //TODO
                    nnz_r: 0.0, // TODO
                    nnz_c: 0.0, // TODO
                }
            }

            Plan::Alt([a, b]) => {
                let _ca = costs(*a);
                let _cb = costs(*b);
                todo!()
            }

            Plan::Star([a]) => {
                let _ca = costs(*a);
                todo!()
            }

            Plan::LStar([a, b]) => {
                let _ca = costs(*a);
                let _cb = costs(*b);
                todo!()
            }

            Plan::RStar([a, b]) => {
                let _ca = costs(*a);
                let _cb = costs(*b);
                todo!()
            }
        }
    }
}

pub struct CardinalityEstFn;
impl CostFunction<Plan> for CardinalityEstFn {
    type Cost = usize;
    fn cost<C>(&mut self, enode: &Plan, mut cardinalities: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        match enode {
            Plan::Label(meta) => meta.nvals,
            Plan::Seq(_args) => todo!(),
            Plan::Alt(args) => cardinalities(args[0]) + cardinalities(args[1]),
            Plan::Star(_args) => todo!(),
            Plan::LStar(_args) => todo!(),
            Plan::RStar(_args) => todo!(),
        }
    }
}

pub struct CostFn;
impl CostFunction<Plan> for CostFn {
    type Cost = f64;
    fn cost<C>(&mut self, enode: &Plan, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        match enode {
            Plan::Label(_meta) => 0.0,
            Plan::Seq(args) =>
            /* costs(args[0]) + costs(args[1]) +*/
            {
                todo!()
            }
            Plan::Alt(args) =>
            /* costs(args[0]) + costs(args[1]) +*/
            {
                todo!()
            }
            Plan::Star(args) =>
            /* costs(args[0]) +*/
            {
                todo!()
            }
            Plan::LStar(args) => todo!(),
            Plan::RStar(args) => todo!(),
        }
    }
}

pub struct StupidCostFn;
impl CostFunction<Plan> for StupidCostFn {
    type Cost = f64;
    fn cost<C>(&mut self, enode: &Plan, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        match enode {
            Plan::Label(meta) => meta.nvals as f64,
            Plan::Seq(args) => (costs(args[0]) + costs(args[1])).powf(1.1),
            Plan::Alt(args) => (costs(args[0]) + costs(args[1])).powf(1.1),
            _ => todo!(),
        }
    }
}

pub struct _AdjustedCostFn<CostFn: CostFunction<Plan, Cost = f64>>(
    pub CostFn,
    pub HashMap<Plan, f64>,
);
impl<CostFn: CostFunction<Plan, Cost = f64>> CostFunction<Plan> for _AdjustedCostFn<CostFn> {
    type Cost = f64;

    fn cost<C>(&mut self, enode: &Plan, costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let _AdjustedCostFn(orig_cost_fn, adjusts) = self;
        match adjusts.get(enode) {
            Some(cost) => cost.clone(),
            None => orig_cost_fn.cost(enode, costs),
        }
    }
}

impl core::fmt::Debug for grb::Matrix {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Matrix").finish()
    }
}

#[derive(Clone, Debug)]
pub struct WanderCostIntm {
    nvals: usize,
    cost: f64,
    mat: grb::Matrix,
}

impl PartialEq for WanderCostIntm {
    // Required method
    fn eq(&self, other: &WanderCostIntm) -> bool {
        self.cost.eq(&other.cost)
    }
}
impl PartialOrd for WanderCostIntm {
    fn partial_cmp(&self, other: &WanderCostIntm) -> Option<std::cmp::Ordering> {
        self.cost.partial_cmp(&other.cost)
    }
}

pub struct WanderCostFn<'a> {
    pub graph: &'a Graph,
}
impl<'a> CostFunction<Plan> for WanderCostFn<'a> {
    type Cost = WanderCostIntm;

    fn cost<C>(&mut self, enode: &Plan, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        match enode {
            Plan::Seq([lhs, rhs]) => {
                let mut smat = grb::Matrix::null();
                let lhs = costs(*lhs);
                let rhs = costs(*rhs);
                let mut nvals: usize = 0;
                unsafe {
                    LAGraph_RPQMatrix_Seq(
                        lhs.mat,
                        rhs.mat,
                        (&mut smat) as *mut grb::Matrix,
                        (&mut nvals) as *mut usize,
                    );
                }
                WanderCostIntm {
                    mat: smat,
                    nvals: nvals * 8usize.pow(3),
                    cost: lhs.cost + rhs.cost + ((lhs.nvals as f64) + (rhs.nvals as f64)).powf(1.4),
                }
            }
            Plan::Alt([lhs, rhs]) => {
                let mut smat = grb::Matrix::null();
                let lhs = costs(*lhs);
                let rhs = costs(*rhs);
                let mut nvals: usize = 0;
                unsafe {
                    LAGraph_RPQMatrix_Alt(
                        lhs.mat,
                        rhs.mat,
                        (&mut smat) as *mut grb::Matrix,
                        (&mut nvals) as *mut usize,
                    );
                }
                WanderCostIntm {
                    mat: smat,
                    nvals: nvals * 8usize.pow(3),
                    cost: lhs.cost + rhs.cost + ((lhs.nvals as f64) + (rhs.nvals as f64)),
                }
            }
            Plan::Label(meta) => {
                let mat = (*self.graph).mats.get(&meta.name).unwrap().clone();
                let mut smat = grb::Matrix::null();
                unsafe {
                    LAGraph_RPQMatrix_ExtractRandom(mat, (&mut smat) as *mut grb::Matrix, 42);
                }
                WanderCostIntm {
                    nvals: meta.nvals,
                    mat: smat,
                    cost: 0.0,
                }
            }
            _ => {
                todo!()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    pub struct CostFn;
    impl CostFunction<Plan> for CostFn {
        type Cost = f64;
        fn cost<C>(&mut self, enode: &Plan, mut costs: C) -> Self::Cost
        where
            C: FnMut(Id) -> Self::Cost,
        {
            match enode {
                Plan::Label(meta) => meta.nvals as f64,
                Plan::Seq(args) => costs(args[0]).min(costs(args[1])).powf(1.1),
                Plan::Alt(args) => costs(args[0]).min(costs(args[1])).powf(1.1),
                Plan::Star(args) => costs(args[0]).powi(2),
                Plan::LStar(args) => costs(args[0]) * costs(args[1]),
                Plan::RStar(args) => costs(args[0]) * costs(args[1]),
            }
        }
    }

    fn test_simplify(s: String) -> String {
        let expr = s.parse().unwrap();
        let runner = Runner::default().with_expr(&expr).run(&make_rules());
        let cost_func = CostFn;
        let extractor = Extractor::new(&runner.egraph, cost_func);
        extractor.find_best(runner.roots[0]).1.to_string()
    }

    #[test]
    fn test_basic_seq_1() {
        expect![[r#"(/ "(-, 1)" (/ "(-, 2)" (/ "(-, 3)" "(-, 4)")))"#]]
            .assert_eq(test_simplify("(/ (/ (/ 1 2) 3) 4)".to_string()).as_str());
    }

    #[test]
    fn test_basic_seq_2() {
        expect![[r#"(/ "(-, 4)" (/ "(-, 3)" (/ "(-, 2)" "(-, 1)")))"#]]
            .assert_eq(test_simplify("(/ (/ (/ 4 3) 2) 1)".to_string()).as_str());
    }

    #[test]
    fn test_basic_alt_1() {
        expect![[r#"(| "(-, 2)" (| "(-, 4)" (| "(-, 1)" "(-, 3)")))"#]]
            .assert_eq(test_simplify("(| (| (| 1 2) 3) 4)".to_string()).as_str());
    }

    #[test]
    fn test_basic_alt_2() {
        expect![[r#"(| "(-, 3)" (| "(-, 1)" (| "(-, 4)" "(-, 2)")))"#]]
            .assert_eq(test_simplify("(| (| (| 4 3) 2) 1)".to_string()).as_str());
    }
}
