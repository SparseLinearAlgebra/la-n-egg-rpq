mod eval;
mod graph;
mod grb;
mod plan;
// #[cfg(debug_assertions)]
mod pprint;
mod query;

use crate::{
    eval::{eval, LAGraph_Init},
    plan::{make_rules, CardinalityCostFn, NnzCostFn, RandomCostFn, WanderCostFn},
    query::Query,
};
use egg::{RecExpr, Runner};
use graph::Graph;
use plan::Plan;
// #[cfg(debug_assertions)]
use pprint::pretty;
use std::{ops::Div, path::Path, time::Duration, collections::HashSet};

#[cfg(debug_assertions)]
macro_rules! dprintln {
    ($($arg:tt)*) => {
        println!($($arg)*);
    };
}

// #[cfg(debug_assertions)]
fn measure_plan_min(
    graph: &Graph,
    plan: &RecExpr<Plan>,
    repeats: usize,
) -> Option<(usize, Duration)> {
    let mut best_time = Duration::MAX;
    let mut best_answer = None;

    for _ in 0..repeats {
        let start = std::time::Instant::now();
        let answer = eval(graph, plan.clone()).ok()?;
        let elapsed = start.elapsed();

        if elapsed < best_time {
            best_time = elapsed;
            best_answer = Some(answer);
        }
    }

    Some((best_answer?, best_time))
}

fn collect_random_unique_plans(
    expr: &RecExpr<Plan>,
    target_unique: usize,
    max_attempts: usize,
    max_stale: usize,
) -> Vec<RecExpr<Plan>> {
    let rules = make_rules();

    let runner = Runner::default()
        .with_explanations_disabled()
        .with_expr(expr)
        .run(&rules);

    let root = runner.roots[0];
    let mut seen = HashSet::<String>::new();
    let mut plans = Vec::<RecExpr<Plan>>::new();
    let mut stale = 0usize;

    for _ in 0..max_attempts {
        let extractor = egg::Extractor::new(&runner.egraph, RandomCostFn);
        let (_, plan) = extractor.find_best(root);

        let key = pretty(&plan);
        if seen.insert(key) {
            plans.push(plan);
            stale = 0;

            if plans.len() >= target_unique {
                break;
            }
        } else {
            stale += 1;
            if stale >= max_stale {
                break;
            }
        }
    }

    plans
}

fn find_best_random_plan(
    graph: &Graph,
    expr: &RecExpr<Plan>,
    target_unique: usize,
    max_attempts: usize,
    max_stale: usize,
    repeats_per_plan: usize,
) -> Option<(RecExpr<Plan>, usize, Duration, usize)> {
    let plans = collect_random_unique_plans(expr, target_unique, max_attempts, max_stale);

    let found_unique = plans.len();

    plans
        .into_iter()
        .filter_map(|plan| {
            let (answer, time) = measure_plan_min(graph, &plan, repeats_per_plan)?;
            Some((plan, answer, time, found_unique))
        })
        .min_by_key(|(_, _, time, _)| *time)
}

fn debug_compare_with_random(
    graph: &Graph,
    expr: &RecExpr<Plan>,
    chosen_plan: &RecExpr<Plan>,
    chosen_time: Duration,
) {
    let target_unique = 50;
    let max_attempts = 200;
    let max_stale = 100;
    let repeats_per_plan = 2;

    let chosen = measure_plan_min(graph, chosen_plan, repeats_per_plan);
    let chosen_time = chosen.map(|(_, t)| t).unwrap_or(chosen_time);

    let best_random = find_best_random_plan(
        graph,
        expr,
        target_unique,
        max_attempts,
        max_stale,
        repeats_per_plan,
    );

    if let Some((best_plan, _, best_time, found_unique)) = best_random {
        println!("{:?}#{:?}", chosen_time, pretty(chosen_plan));
        println!("{:?}#{:?}", best_time, pretty(&best_plan));
        // println!("Unique random plans found: {}", found_unique);
        println!(
            "{:.2}",
            chosen_time.as_secs_f64() / best_time.as_secs_f64()
        );
    } else {
        println!("Chosen: {:?} {:?}", chosen_time, pretty(chosen_plan));
        println!("Best random: not found");
    }
}

#[cfg(not(debug_assertions))]
macro_rules! dprintln {
    ($($arg:tt)*) => {};
}

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
fn run_nnz<'a>(
    graph: &'a Graph,
    expr: &'a RecExpr<Plan>,
) -> Option<(RecExpr<Plan>, usize, Duration)> {
    let rules = make_rules();

    // planning
    let _runner_start = std::time::Instant::now();
    let runner = Runner::default()
        .with_explanations_disabled()
        .with_expr(expr)
        .run(&rules);
    let _runner_time = _runner_start.elapsed();
    // planning

    // This should perform a heat up.
    (0..10).for_each(|_| {
        let extractor = egg::Extractor::new(&runner.egraph, RandomCostFn);
        let (_, plan) = extractor.find_best(runner.roots[0]);
        let _ = eval(graph, plan);
    });

    // extract
    let _extract_start = std::time::Instant::now();
    let extractor = egg::Extractor::new(&runner.egraph, NnzCostFn);
    let (_, plan) = extractor.find_best(runner.roots[0]);
    let _extract_time = _extract_start.elapsed();
    // extract

    // execution
    let _start = std::time::Instant::now();
    let answer = eval(graph, plan.clone()).ok()?;
    // execution

    let _eval_time = _start.elapsed();
    // #[cfg(debug_assertions)]
    debug_compare_with_random(graph, expr, &plan, _eval_time);
    // dprintln!(
    //     "runner time: {:.2}\nextract time: {}\neval time: {} \nplanning time: {}\ntotal time: {}",
    //     _runner_time.as_secs_f64(),
    //     _extract_time.as_secs_f64(),
    //     _eval_time.as_secs_f64(),
    //     _runner_time.as_secs_f64() + _extract_time.as_secs_f64(),
    //     _runner_time.as_secs_f64() + _extract_time.as_secs_f64() + _eval_time.as_secs_f64(),
    // );
    Some((plan, answer, _eval_time))
}

fn run_cardinality<'a>(
    graph: &'a Graph,
    expr: &'a RecExpr<Plan>,
) -> Option<(RecExpr<Plan>, usize, Duration)> {
    let rules = make_rules();

    // planning
    let _runner_start = std::time::Instant::now();
    let runner = Runner::default()
        .with_explanations_disabled()
        .with_expr(expr)
        .run(&rules);
    let _runner_time = _runner_start.elapsed();
    // planning

    // This should perform a heat up.
    // (0..10).for_each(|_| {
    //     let extractor = egg::Extractor::new(&runner.egraph, RandomCostFn);
    //     let (_, plan) = extractor.find_best(runner.roots[0]);
    //     let _ = eval(graph, plan);
    // });

    // extract
    let _extract_start = std::time::Instant::now();
    let extractor = egg::Extractor::new(
        &runner.egraph,
        CardinalityCostFn {
            n: graph.size as f64,
            star_penalty: 50.0,
            lr_multiplier: 5.0,
        },
    );
    let (_, plan) = extractor.find_best(runner.roots[0]);
    let _extract_time = _extract_start.elapsed();
    // extract

    // execution
    let _start = std::time::Instant::now();
    let answer = eval(graph, plan.clone()).ok()?;
    // execution

    let _eval_time = _start.elapsed();

    // #[cfg(debug_assertions)]
    debug_compare_with_random(graph, expr, &plan, _eval_time);
    // dprintln!(
    //     "runner time: {:?}\nextract time: {:?}\neval time: {:?} \nplanning time: {:?}\ntotal time: {:?}",
    //     _runner_time,
    //     _extract_time,
    //     _eval_time,
    //     _runner_time + _extract_time,
    //     _runner_time + _extract_time + _eval_time,
    // );
    Some((plan, answer, _eval_time))
}

fn run_simple<'a>(
    graph: &'a Graph,
    expr: &'a RecExpr<Plan>,
) -> Option<(RecExpr<Plan>, usize, Duration)> {
    // let rules = make_rules();

    // planning
    // let _runner_start = std::time::Instant::now();
    // let runner = Runner::default()
    //     .with_explanations_disabled()
    //     .with_expr(expr)
    //     .run(&rules);
    // let _runner_time = _runner_start.elapsed();
    // // planning

    // // This should perform a heat up.
    // (0..10).for_each(|_| {
    //     let extractor = egg::Extractor::new(&runner.egraph, RandomCostFn);
    //     let (_, plan) = extractor.find_best(runner.roots[0]);
    //     let _ = eval(graph, plan);
    // });

    // // extract
    // let _extract_start = std::time::Instant::now();
    // let extractor = egg::Extractor::new(
    //     &runner.egraph,
    //     CardinalityCostFn {
    //         n: graph.size as f64,
    //         star_penalty: 50.0,
    //         lr_multiplier: 5.0,
    //     },
    // );
    // let (_, plan) = extractor.find_best(runner.roots[0]);
    // let _extract_time = _extract_start.elapsed();
    // extract

    // execution
    let _start = std::time::Instant::now();
    let answer = eval(graph, expr.clone()).ok()?;
    // execution

    let _eval_time = _start.elapsed();

    // #[cfg(debug_assertions)]
    // debug_compare_with_random(graph, expr, &plan, _eval_time);
    // dprintln!(
    //     "runner time: {:?}\nextract time: {:?}\neval time: {:?} \nplanning time: {:?}\ntotal time: {:?}",
    //     _runner_time,
    //     _extract_time,
    //     _eval_time,
    //     _runner_time + _extract_time,
    //     _runner_time + _extract_time + _eval_time,
    // );
    Some((expr.clone(), answer, _eval_time))
}

fn run_wander<'a>(
    graph: &'a Graph,
    expr: &'a RecExpr<Plan>,
) -> Option<(RecExpr<Plan>, usize, Duration)> {
    let rules = make_rules();

    // planning
    let _runner_start = std::time::Instant::now();
    let runner = Runner::default()
        .with_explanations_disabled()
        .with_expr(expr)
        .run(&rules);
    let _runner_time = _runner_start.elapsed();
    // planning

    // extract
    let _extract_start = std::time::Instant::now();
    let extractor = egg::Extractor::new(&runner.egraph, WanderCostFn { graph: graph });
    let (_, plan) = extractor.find_best(runner.roots[0]);
    let _extract_time = _extract_start.elapsed();
    // extract

    // execution
    let _start = std::time::Instant::now();
    let answer = eval(graph, plan.clone()).ok()?;
    // execution

    let _eval_time = _start.elapsed();
    dprintln!(
        "runner time: {:?}\nextract time: {:?}\neval time: {:?} \nplanning time: {:?}\ntotal time: {:?}",
        _runner_time.as_nanos(),
        _extract_time.as_nanos(),
        _eval_time.as_nanos(),
        _runner_time.as_nanos() + _extract_time.as_nanos(),
        _runner_time.as_nanos() + _extract_time.as_nanos() + _eval_time.as_nanos(),
    );
    Some((plan, answer, _eval_time))
}

// fn run_wander<'a>(
//     graph: &'a Graph,
//     expr: &'a RecExpr<Plan>,
// ) -> Option<(RecExpr<Plan>, usize, Duration)> {
//     let rules = make_rules();

//     let runner = Runner::default()
//         .with_explanations_disabled()
//         .with_expr(expr)
//         .run(&rules);

//     let costfn = WanderCostFn { graph: graph };
//     let extractor = egg::Extractor::new(&runner.egraph, costfn);
//     let (cost, plan) = extractor.find_best(runner.roots[0]);
//     let start = std::time::Instant::now();
//     // TODO: check answers.
//     let answer = eval(graph, plan.clone()).ok()?;
//     Some((plan, answer, start.elapsed()))
// }

// fn run_stupid<'a>(
//     graph: &'a Graph,
//     expr: &'a RecExpr<Plan>,
// ) -> Option<(RecExpr<Plan>, usize, Duration)> {
//     let stupid_rules = make_stupid_rules();

//     let runner = Runner::default()
//         .with_explanations_disabled()
//         .with_expr(expr)
//         .run(&stupid_rules);

//     let extractor = egg::Extractor::new(&runner.egraph, StupidCostFn);
//     let (_, plan) = extractor.find_best(runner.roots[0]);
//     let start = std::time::Instant::now();
//     // TODO: check answers.
//     let answer = eval(graph, plan.clone()).ok()?;
//     Some((plan, answer, start.elapsed()))
// }

fn main() {
    unsafe {
        let res = LAGraph_Init(std::ptr::null_mut());
        assert_eq!(res, 0);
    }

    let graph_path = std::env::args().nth(1).unwrap();
    let graph_path = Path::new(&graph_path);
    let mut graph = graph::load_dir(graph_path).expect("unable to load graph");

    let queries_path = std::env::args().nth(2).unwrap();
    let queries_path = Path::new(&queries_path);

    let cost_fun_type = std::env::args().nth(3).unwrap();
    let mut query_num = 1;
    read_queries(queries_path).into_iter().for_each(|query| {
        // println!("Running {:?}", query);
        let expr = graph.run(query.clone());
        match cost_fun_type.as_str() {
            "nnz" => match expr {
                Ok(expr) => {
                    let result: Option<(RecExpr<Plan>, usize, Duration)> = run_nnz(&graph, &expr);
                    match result {
                        Some((_plan, ans, duration)) => {
                            println!("{};{};{:?}", query_num, ans, duration.as_nanos());
                        }
                        None => {
                            println!("no result");
                        }
                    }
                }
                Err(msg) => {
                    println!("unable to execute query: {}", msg);
                }
            },
            "cardinality" => match expr {
                Ok(expr) => {
                    let result: Option<(RecExpr<Plan>, usize, Duration)> =
                        run_cardinality(&graph, &expr);
                    match result {
                        Some((_plan, ans, duration)) => {
                            // println!("{};{};{:?}", query_num, ans, duration.as_nanos());
                        }
                        None => {
                            println!("no result");
                        }
                    }
                }
                Err(msg) => {
                    println!("unable to execute query: {}", msg);
                }
            },
            "wander" => match expr {
                Ok(expr) => {
                    let result: Option<(RecExpr<Plan>, usize, Duration)> =
                        run_wander(&graph, &expr);
                    match result {
                        Some((_plan, ans, duration)) => {
                            println!("{};{};{:?}", query_num, ans, duration.as_nanos());
                        }
                        None => {
                            println!("no result");
                        }
                    }
                }
                Err(msg) => {
                    println!("unable to execute query: {}", msg);
                }
            },
            "simple" => match expr {
                Ok(expr) => {
                    let result: Option<(RecExpr<Plan>, usize, Duration)> =
                        run_simple(&graph, &expr);
                    match result {
                        Some((_plan, ans, duration)) => {
                            println!("{};{};{:?}", query_num, ans, duration.as_nanos());
                        }
                        None => {
                            println!("no result");
                        }
                    }
                }
                Err(msg) => {
                    println!("unable to execute query: {}", msg);
                }
            },
            "random" => {
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
            }
            &_ => todo!(),
        }
        query_num += 1;
    });
}

#[cfg(test)]
mod tests {}
