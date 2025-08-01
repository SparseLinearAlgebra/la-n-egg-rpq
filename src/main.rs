mod eval;
mod graph;
mod grb;
mod plan;
mod query;

use crate::{
    eval::{eval, LAGraph_Init},
    plan::{make_rules, RandomCostFn},
    query::Query,
};
use egg::{RecExpr, Runner};
use graph::Graph;
use plan::Plan;
use std::{ops::Div, path::Path, time::Duration};

/// Read queries from file.
///
/// The file should contain lines satisfying the following pattern: `<number>,<src> <pattern> <dest>`.
///
/// # Query file example
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

fn run_random<'a>(
    graph: &'a Graph,
    runs: u32,
    expr: &'a RecExpr<Plan>,
) -> impl Iterator<Item = (RecExpr<Plan>, usize, Duration)> + 'a {
    let rules = make_rules();

    let runner = Runner::default()
        .with_explanations_disabled()
        .with_expr(expr)
        .run(&rules);

    // This should perform a heat up.
    (0..runs).for_each(|_| {
        let extractor = egg::Extractor::new(&runner.egraph, RandomCostFn);
        let (_, plan) = extractor.find_best(runner.roots[0]);
        let _ = eval(graph, plan);
    });

    (0..runs).filter_map(move |_| {
        let extractor = egg::Extractor::new(&runner.egraph, RandomCostFn);
        let (_, plan) = extractor.find_best(runner.roots[0]);
        let start = std::time::Instant::now();
        // TODO: check answers.
        let answer = eval(graph, plan.clone()).ok()?;
        Some((plan, answer, start.elapsed()))
    })
}

fn main() {
    unsafe {
        let res = LAGraph_Init(std::ptr::null_mut());
        assert_eq!(res, 0);
    }

    let graph_path = std::env::args().nth(1).unwrap();
    let graph_path = Path::new(&graph_path);
    let graph = graph::load_dir(graph_path).expect("unable to load graph");

    let queries_path = std::env::args().nth(2).unwrap();
    let queries_path = Path::new(&queries_path);

    read_queries(queries_path).into_iter().for_each(|query| {
        println!("Running {:?}", query);
        let expr = graph.run(query.clone());
        match expr {
            Ok(expr) => {
                let runs: u32 = 1000;
                let results: Vec<(RecExpr<Plan>, usize, Duration)> =
                    run_random(&graph, runs, &expr).collect();
                let first_n_runs = runs / 100;
                println!("Stats for {:?}", query);
                println!("    First {:?} runs", first_n_runs);
                results
                    .iter()
                    .take(first_n_runs.try_into().unwrap())
                    .for_each(|(plan, ans, duration)| {
                        println!("    - {:?} {} {}", duration, plan, ans);
                    });
                //results.sort_by_key(|(_plan, _ans, duration)| duration);
                let (best_plan, _, best_time) = results
                    .iter()
                    .min_by_key(|(_plan, _ans, duration)| duration)
                    .unwrap();
                let (worst_plan, _, worst_time) = results
                    .iter()
                    .max_by_key(|(_plan, _ans, duration)| duration)
                    .unwrap();
                let mean_time = results
                    .iter()
                    .map(|(_plan, _ans, duration)| duration)
                    .sum::<Duration>()
                    .div(runs);
                //let (_, _, median_time) = results[results.len() / 2].clone();

                println!("    Best {:?}: {}", best_time, best_plan);
                println!("    Worst {:?}: {}", worst_time, worst_plan);
                println!("    Mean: {:?}", mean_time);
                //println!("    Median: {:?}", median_time);
                println!();
            }
            Err(msg) => {
                println!("unable to execute query: {}", msg);
            }
        }
    });
}

#[cfg(test)]
mod tests {}
