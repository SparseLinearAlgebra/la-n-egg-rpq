mod graph;
mod plan;
mod query;

use egg::Runner;
use std::path::Path;

use crate::{
    plan::{make_rules, SillyCostFn},
    query::Query,
};

fn main() {
    let graph_path = std::env::args().nth(1).unwrap();
    let graph_path = Path::new(&graph_path);
    let graph = graph::load_dir(graph_path).expect("unable to load graph");

    let rules = make_rules();

    let queries_path = std::env::args().nth(2).unwrap();
    let queries_path = Path::new(&queries_path);
    std::fs::read_to_string(queries_path)
        .expect("unable to load queries")
        .lines()
        .filter_map(|line| Some(line.split(',').nth(1)?.to_string()))
        .filter_map(|query| query.parse::<Query>().ok())
        .for_each(|query| {
            let expr = graph.run(query);
            let runner = Runner::default()
                .with_explanations_disabled()
                .with_expr(&expr)
                .run(&rules);
            let extractor = egg::Extractor::new(&runner.egraph, SillyCostFn);
            let (best_cost, best) = extractor.find_best(runner.roots[0]);
            println!(" {:?} {:?}", best_cost, best);
        });
}
