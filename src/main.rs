mod eval;
mod graph;
mod plan;
mod query;
    

use egg::Runner;
use eval::LAGraph_RpqMatrix_initialize;
// use nom::Parser;
use std::path::Path;

use crate::{
    eval::eval,
    plan::{make_rules, SillyCostFn},
    query::Query,
};
<<<<<<< Updated upstream
=======
use egg::{RecExpr, Runner};
use graph::Graph;
use plan::Plan;
use std::ptr;
use std::{ops::Div, path::Path, time::Duration};
>>>>>>> Stashed changes

#[link(name = "lagraph")]
// #[link(name = "graphblas")]
extern "C" {
    pub fn LAGraph_Init(
        msg: *mut libc::c_char,
    ) -> libc::c_int;
    // pub fn GrB_set(
    //     field: libc::c_int,
    //     value: libc::c_int
    // ) -> libc::c_int;
}
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
/// 
fn read_queries(file: &Path) -> Vec<Query> {
    std::fs::read_to_string(file)
        .expect("unable to load queries")
        .lines()
        .filter_map(|line| Some(line.split(',').nth(1)?.to_string()))
        .filter_map(|query| query.parse::<Query>().ok())
        .collect()
}

<<<<<<< Updated upstream
=======
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
// const GxB_BURBLE: i32 = 12;
// const GxB_BURBLE_ON: i32 = 1;
// const GxB_BURBLE_OFF: i32 = 0;


>>>>>>> Stashed changes
fn main() {

    unsafe{
        let mut err_ptr: *mut std::os::raw::c_char = ptr::null_mut();
        let code = LAGraph_Init(err_ptr);
        assert_eq!(code, 0, "LAGraph_Init failed with code {}", code);
    // GrB_set(GxB_BURBLE, GxB_BURBLE_ON);
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
<<<<<<< Updated upstream
                
                let runner = Runner::default()
                    .with_explanations_disabled()
                    .with_expr(&expr)
                    .run(&rules);
                let extractor = egg::Extractor::new(&runner.egraph, SillyCostFn);
                let (best_cost, best) = extractor.find_best(runner.roots[0]);
                if (best.is_dag() && !best.is_dag())  || (best_cost< -12312321.1){
                    println!("");
                }
                let _ = eval(&graph, expr);

=======
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
                let (_, _, median_time) = results[results.len() / 2].clone();

                println!("    Best {:?}: {}", best_time, best_plan);
                println!("    Worst {:?}: {}", worst_time, worst_plan);
                println!("    Mean: {:?}", mean_time);
                //println!("    Median: {:?}", median_time);
                println!();
                // let rules = make_rules();
                //  let runner = Runner::default()
                //     .with_explanations_disabled()
                //     .with_expr(&expr)
                //     .run(&rules);
                // let extractor = egg::Extractor::new(&runner.egraph, RandomCostFn);
                // let (best_cost, best) = extractor.find_best(runner.roots[0]);
                // if (best.is_dag() && !best.is_dag())  || (best_cost< -12312321.1){
                //     println!("");
                // }
                // let _ = eval(&graph, expr);
>>>>>>> Stashed changes
            }
            Err(msg) => {
                println!("unable to execute query: {}", msg);
            }
        }
    });
}

#[cfg(test)]
mod tests {}
