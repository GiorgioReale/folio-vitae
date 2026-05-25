#![allow(clippy::expect_used, clippy::unwrap_used)]

use assert_cmd::Command;
use assert_fs::{TempDir, prelude::*};
use predicates::prelude::*;

#[test]
fn init_creates_scaffold_files() {
    let temp = TempDir::new().expect("temporary directory");

    Command::new(assert_cmd::cargo::cargo_bin!("folio-vitae"))
        .current_dir(&temp)
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized folio-vitae project"));

    temp.child("data/data.it.yaml").assert(predicate::path::is_file());
    temp.child("data/data.en.yaml").assert(predicate::path::is_file());
    temp.child(".env.example").assert(predicate::path::is_file());
}

#[test]
fn dev_help_lists_port_options() {
    Command::new(assert_cmd::cargo::cargo_bin!("folio-vitae"))
        .arg("dev")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--main-port"))
        .stdout(predicate::str::contains("--cv-port"))
        .stdout(predicate::str::contains("--env-file"));
}

#[test]
fn root_help_mentions_runtime_commands() {
    Command::new(assert_cmd::cargo::cargo_bin!("folio-vitae"))
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("dev"))
        .stdout(predicate::str::contains("prod"))
        .stdout(predicate::str::contains("init"));
}
