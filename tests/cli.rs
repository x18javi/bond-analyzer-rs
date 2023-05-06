use assert_cmd::Command;
use predicates::prelude::*;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

const PRG: &str = "bond-analyzer";

#[ignore]
#[test]
fn initial_test() -> TestResult {
    let _msg = "maturity_date_arg: 2025-01-31";

    Command::cargo_bin(PRG)?
        .args(&[
            "-c",
            "1.375",
            "-d",
            "ACT/ACT",
            "-f",
            "2",
            "-m",
            "2025-01-31",
        ])
        .assert()
        .success();
    //.failure()
    //.stderr(predicate::str::contains(msg));

    Ok(())
}
