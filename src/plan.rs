use std::{fmt::Display, str::FromStr};

use egg::*;

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
    "*r" = LStar([egg::Id; 2]),
    "l*" = RStar([egg::Id; 2]),
} }

pub fn make_rules() -> Vec<egg::Rewrite<Plan, ()>> {
    vec![
        rewrite!("assoc-sec-1"; "(/ ?a (/ ?b ?c))" => "(/ (/ ?a ?b) ?c)"),
        rewrite!("assoc-sec-2"; "(/ (/ ?a ?b) ?c)" => "(/ ?a (/ ?b ?c))"),
        rewrite!("commute-alt"; "(| ?a ?b)" => "(| ?b ?a)"),
        rewrite!("assoc-alt"; "(| ?a (| ?b ?c))" => "(| (| ?a ?b) ?c)"),
        rewrite!("distribute-1"; "(/ ?a (| ?b ?c))" => "(| (/ ?a ?b) (/ ?a ?c))"),
        rewrite!("distribute-2"; "(/ (| ?a ?b) ?c)" => "(| (/ ?a ?c) (/ ?b ?c))"),
        rewrite!("build-lstar"; "(/ ?a (* ?b))" => "(l* ?a ?b)"),
        rewrite!("build-rstar"; "(/ (* ?a) ?b)" => "(*r ?a ?b)"),
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

pub struct CardCost {
    pub score: f64,
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
                    nnz_r, // TODO
                    nnz_c, // TODO
                }
            }

            Plan::Alt([a, b]) => {
                let _ca = costs(*a);
                let _cb = costs(*b);
                panic!()
            }

            Plan::Star([a]) => {
                let _ca = costs(*a);
                panic!()
            }

            Plan::LStar([a, b]) => {
                let _ca = costs(*a);
                let _cb = costs(*b);
                panic!()
            }

            Plan::RStar([a, b]) => {
                let _ca = costs(*a);
                let _cb = costs(*b);
                panic!()
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
