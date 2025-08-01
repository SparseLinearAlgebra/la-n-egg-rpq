use std::{fmt::Display, str::FromStr};

use egg::*;

#[derive(Clone, Hash, Ord, Eq, PartialEq, PartialOrd, Debug)]
pub struct LabelMeta {
    pub name: String,
    pub nvals: usize,
}

impl FromStr for LabelMeta {
    type Err = <usize as FromStr>::Err;
    // This is needed for the builtin egg parser. Only used in tests.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(LabelMeta {
            name: "-".to_string(),
            nvals: s.parse()?,
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
        rewrite!("assoc-alt"; "(| ?a (| ?b ?c))" => "(| (| ?a ?b) ?c)"),
        rewrite!("commute-alt"; "(| ?a ?b)" => "(| ?b ?a)"),
        rewrite!("distribute"; "(/ ?a (| ?b ?c))" => "(| (/ ?a ?b) (/ ?a ?c))"),
        rewrite!("build-lstar"; "(/ ?a (* ?b))" => "(l* ?a ?b)"),
        rewrite!("build-rstar"; "(/ (* ?a) ?b)" => "(*r ?a ?b)"),
    ]
}

pub struct SillyCostFn;
impl CostFunction<Plan> for SillyCostFn {
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

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    fn test_simplify(s: String) -> String {
        let expr = s.parse().unwrap();
        let runner = Runner::default().with_expr(&expr).run(&make_rules());
        let cost_func = SillyCostFn;
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
        expect![[r#"(| "(-, 1)" (| "(-, 2)" (| "(-, 3)" "(-, 4)")))"#]]
            .assert_eq(test_simplify("(| (| (| 1 2) 3) 4)".to_string()).as_str());
    }

    #[test]
    fn test_basic_alt_2() {
        expect![[r#"(| "(-, 4)" (| "(-, 3)" (| "(-, 2)" "(-, 1)")))"#]]
            .assert_eq(test_simplify("(| (| (| 4 3) 2) 1)".to_string()).as_str());
    }
}
