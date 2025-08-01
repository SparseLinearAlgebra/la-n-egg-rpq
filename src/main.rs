mod eval;
mod graph;
mod plan;
mod query;

use egg::Runner;
use eval::LAGraph_RpqMatrix_initialize;
use std::path::Path;

use crate::{
    eval::eval,
    plan::{make_rules, SillyCostFn},
    query::Query,
};

/// Read queries from file.
///
/// The file should contain lines satisfying the following pattern: `<number>,<src> <pattern> <dest>`.
///
/// # Query file example
///
/// ```
/// 1,?sub <references>/<cite>/<creator> ?obj
/// 2,?sub (<coauthor>)+ <Fiorenza_Summerset>
/// 3,<Article1659> (<references>/<cite>)* ?obj
/// ```
fn read_queries(file: &Path) -> Vec<Query> {
    std::fs::read_to_string(file)
        .expect("unable to load queries")
        .lines()
        .filter_map(|line| Some(line.split(',').nth(1)?.to_string()))
        .filter_map(|query| query.parse::<Query>().ok())
        .collect()
}

fn main() {
    unsafe {
        LAGraph_RpqMatrix_initialize();
    }

    let graph_path = std::env::args().nth(1).unwrap();
    let graph_path = Path::new(&graph_path);
    let graph = graph::load_dir(graph_path).expect("unable to load graph");

    let rules = make_rules();

    let queries_path = std::env::args().nth(2).unwrap();
    let queries_path = Path::new(&queries_path);

    read_queries(queries_path).into_iter().for_each(|query| {
        let expr = graph.run(query);
        match expr {
            Ok(expr) => {
                let runner = Runner::default()
                    .with_explanations_disabled()
                    .with_expr(&expr)
                    .run(&rules);
                let extractor = egg::Extractor::new(&runner.egraph, SillyCostFn);
                let (best_cost, best) = extractor.find_best(runner.roots[0]);
                println!("{:?} {}", best_cost, best);
                eval(&graph, best);
            }
            Err(msg) => {
                println!("unable to execute query: {}", msg);
            }
        }
    });
}

#[cfg(test)]
mod tests {}
