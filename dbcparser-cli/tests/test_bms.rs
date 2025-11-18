use assert_cmd::prelude::*;
use assert_fs::fixture::PathChild;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn bin_path() -> std::path::PathBuf {
    assert_cmd::cargo::cargo_bin!("dbcparser-cli").to_path_buf()
}

/// Read a file, filtering out the line that contains the auto-generated date comment.
fn filter_content(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
        .lines()
        .filter(|line| !line.contains("// - code generated from"))
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn generates_test_1_bms() {
    let tmp = assert_fs::fixture::TempDir::new_in(env::current_dir().unwrap()).unwrap();
    let dbc = tmp.child("../examples/bms/dbc/BMS.dbc");
    let ref_out = tmp.child("../examples/bms/bms.rs");
    let out = tmp.child("../examples/bms/__bms.rs");
    let config = tmp.child("../examples/bms/__bms_config.yaml");

    Command::new(bin_path())
        .args([
            "-i",
            dbc.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--save-config",
            config.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Generated:"));

    out.assert(predicate::path::exists());

    // Compare the files
    let ref_content = filter_content(ref_out.path());
    let out_content = filter_content(out.path());

    assert_eq!(
        ref_content,
        out_content,
        "Generated output differs from reference {}",
        ref_out.path().display()
    );

    Command::new(bin_path())
        .args(["--config", config.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Generated:"));

    let out_content_2 = filter_content(out.path());

    assert_eq!(
        ref_content,
        out_content_2,
        "Generated output differs from reference {}",
        ref_out.path().display()
    );
}

#[test]
fn generates_test_2_bms() {
    let tmp = assert_fs::fixture::TempDir::new_in(env::current_dir().unwrap()).unwrap();
    let dbc = tmp.child("../examples/bms/dbc/BMS.dbc");
    let ref_out = tmp.child("../examples/bms/bms_whitelist.rs");
    let out = tmp.child("../examples/bms/__bms_whitelist.rs");
    let config = tmp.child("../examples/bms/__bms_whitelist_config.yaml");

    Command::new(bin_path())
        .args([
            "-i",
            dbc.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--whitelist",
            "641",
            "--save-config",
            config.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Generated:"));

    out.assert(predicate::path::exists());

    // Compare the files
    let ref_content = filter_content(ref_out.path());
    let out_content = filter_content(out.path());

    assert_eq!(
        ref_content,
        out_content,
        "Generated output differs from reference {}",
        ref_out.path().display()
    );

    Command::new(bin_path())
        .args(["--config", config.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Generated:"));

    let out_content_2 = filter_content(out.path());

    assert_eq!(
        ref_content,
        out_content_2,
        "Generated output differs from reference {}",
        ref_out.path().display()
    );
}

#[test]
fn generates_test_3_bms() {
    let tmp = assert_fs::fixture::TempDir::new_in(env::current_dir().unwrap()).unwrap();
    let dbc = tmp.child("../examples/bms/dbc/BMS.dbc");
    let ref_out = tmp.child("../examples/bms/bms_blacklist.rs");
    let out = tmp.child("../examples/bms/__bms_blacklist.rs");
    let config = tmp.child("../examples/bms/__bms_blacklist_config.yaml");

    Command::new(bin_path())
        .args([
            "-i",
            dbc.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--blacklist",
            "641",
            "--save-config",
            config.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Generated:"));

    out.assert(predicate::path::exists());

    // Compare the files
    let ref_content = filter_content(ref_out.path());
    let out_content = filter_content(out.path());

    assert_eq!(
        ref_content,
        out_content,
        "Generated output differs from reference {}",
        ref_out.path().display()
    );

    Command::new(bin_path())
        .args(["--config", config.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Generated:"));

    let out_content_2 = filter_content(out.path());

    assert_eq!(
        ref_content,
        out_content_2,
        "Generated output differs from reference {}",
        ref_out.path().display()
    );
}
