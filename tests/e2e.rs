use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;

const COST_FUNCTION: [&str; 2] = ["nnz", "cardinality"];

fn graph_dir() -> PathBuf {
    PathBuf::from("tests/testdata/graph")
}

fn cases_dir() -> PathBuf {
    PathBuf::from("tests/testdata/cases")
}

fn parse_expected(path: &std::path::Path) -> Vec<usize> {
    let file = File::open(path).expect("cannot open expected.txt");
    let reader = BufReader::new(file);

    let mut values = Vec::new();

    for line in reader.lines() {
        let line_data = line.expect("invalid line in expected.txt");
        let line_data = line_data.trim();

        if line_data.is_empty() {
            continue;
        }

        let mut parts = line_data.split(';');

        let _query_id = parts.next().expect("missing id");

        let nnz = parts
            .next()
            .expect("missing value")
            .parse::<usize>()
            .expect("bad number");

        values.push(nnz);
    }

    values
}

fn parse_result(result: &str) -> Vec<usize> {
    let mut values = Vec::new();

    for line in result.lines() {
        let line_data = line.trim();
        let mut parts = line_data.split(';');
        let _query_id = parts.next().expect("missing id");
        let nnz = parts
            .next()
            .expect("missing value")
            .parse::<usize>()
            .expect("bad number");

        values.push(nnz);
    }

    values
}

#[test]
fn end_to_end() {
    let bin = env!("CARGO_BIN_EXE_la-n-egg-rpq");
    let graph = graph_dir();
    let cases = cases_dir();
    for case in fs::read_dir(cases).unwrap() {
        let case = case.unwrap();
        let case_path = case.path();

        let case_name = case_path.file_name().unwrap().to_string_lossy().to_string();

        let queries = case_path.join("queries.txt");
        let expected = case_path.join("expected.txt");

        let expected_values = parse_expected(&expected);

        for cost in COST_FUNCTION {
            let output = Command::new(bin)
                .arg(&graph)
                .arg(&queries)
                .arg(cost)
                .output()
                .unwrap_or_else(|_| panic!("failed to run binary on case {}", case_name));
            if !output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                panic!(
                    "binary failed on case {} cost {}\nstdout:\n{}\nstderr:\n{}",
                    case_name, cost, stdout, stderr
                );
            }

            let stdout = String::from_utf8(output.stdout).unwrap();
            let actual_values = parse_result(&stdout);

            assert_eq!(
                expected_values, actual_values,
                "mismatch in case {} with cost {}",
                case_name, cost
            );
        }
    }
}
