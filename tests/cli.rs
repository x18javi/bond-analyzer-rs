use assert_cmd::Command;
use predicates::prelude::*;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

const PRG: &str = "bond-analyzer";

const RESULT: &str = "+------------+----------+
| Date       | Coupon   |
+------------+----------+
| 2020-07-31 | 0.6875   |
+------------+----------+
| 2021-01-31 | 0.6875   |
+------------+----------+
| 2021-07-31 | 0.6875   |
+------------+----------+
| 2022-01-31 | 0.6875   |
+------------+----------+
| 2022-07-31 | 0.6875   |
+------------+----------+
| 2023-01-31 | 0.6875   |
+------------+----------+
| 2023-07-31 | 0.6875   |
+------------+----------+
| 2024-01-31 | 0.6875   |
+------------+----------+
| 2024-07-31 | 0.6875   |
+------------+----------+
| 2025-01-31 | 100.6875 |
+------------+----------+
+-------------------+--------+
| Metric            | Result |
+-------------------+--------+
| YTM               | 1.396  |
+-------------------+--------+
| Macaulay Duration | 4.794  |
+-------------------+--------+
| Modified Duration | 4.761  |
+-------------------+--------+";

#[test]
fn test_good_bond_output() -> TestResult {
    Command::cargo_bin(PRG)?
        .args(&[
            "-c",
            "1.375",
            "-p",
            "99.974",
            "-m",
            "2025-01-31",
            "-s",
            "2020-02-20",
        ])
        .assert()
        .stdout(predicate::str::contains(RESULT));

    Ok(())
}

#[test]
fn test_good_ytm() -> TestResult {
    Command::cargo_bin(PRG)?
        .args(&[
            "-c",
            "1.375",
            "-p",
            "99.974",
            "-m",
            "2025-01-31",
            "-s",
            "2020-02-20",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("1.396"));

    Ok(())
}

#[test]
fn test_missing_price() -> TestResult {
    Command::cargo_bin(PRG)?
        .args(&["-c", "1.375", "-m", "2025-01-31", "-s", "2020-02-20"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "the following required arguments were not provided:\n  --price <PRICE>",
        ));
    Ok(())
}
