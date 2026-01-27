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
fn test_codegen() {}

fn codegen_test_with_config(dbc_file_path: &str, ref_rs_file_path: &str, extra_args: Vec<&str>) {
    let tmp = assert_fs::fixture::TempDir::new_in(env::current_dir().unwrap()).unwrap();
    let mut dbc = env::current_dir().unwrap();
    dbc.push(dbc_file_path);
    let mut ref_out = env::current_dir().unwrap();
    ref_out.push(ref_rs_file_path);
    let out = tmp.child(ref_out.file_name().unwrap());
    let config =
        tmp.child(format!("__{}_config.yaml", ref_out.file_prefix().unwrap().to_str().unwrap()));

    let mut args = vec![
        "-i",
        dbc.to_str().unwrap(),
        "-o",
        out.path().to_str().unwrap(),
        "--save-config",
        config.path().to_str().unwrap(),
    ];
    args.extend_from_slice(&extra_args[..]);
    Command::new(bin_path())
        .args(args)
        .assert()
        .success()
        .stderr(predicate::str::contains("Generated:"));

    out.assert(predicate::path::exists());

    // Compare the files
    let ref_content = filter_content(ref_out.as_path());
    let out_content = filter_content(out.path());

    assert_eq!(
        ref_content,
        out_content,
        "Generated output differs from reference {}",
        ref_out.display()
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
        ref_out.display()
    );
}

fn codegen_no_error(dbc_file_path: &str) {
    let tmp = assert_fs::fixture::TempDir::new_in(env::current_dir().unwrap()).unwrap();
    let mut dbc = env::current_dir().unwrap();
    dbc.push(dbc_file_path);
    let out = tmp.child("gen.rs");

    Command::new(bin_path())
        .args(vec!["-i", dbc.to_str().unwrap(), "-o", out.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Generated:"));

    out.assert(predicate::path::exists());
}

fn codegen_test_snippet(dbc_file_path: &str, snippet: &str, extra_args: Vec<&str>) {
    let tmp = assert_fs::fixture::TempDir::new_in(env::current_dir().unwrap()).unwrap();
    let mut dbc = env::current_dir().unwrap();
    dbc.push(dbc_file_path);
    let out = tmp.child("__generated.rs");

    let mut args = vec![
        "-i",
        dbc.to_str().unwrap(),
        "-o",
        out.path().to_str().unwrap(),
    ];
    args.extend_from_slice(&extra_args[..]);
    Command::new(bin_path())
        .args(args)
        .assert()
        .success()
        .stderr(predicate::str::contains("Generated:"));

    out.assert(predicate::path::exists());

    let content = fs::read_to_string(out.path()).expect(&format!("Error reading file {}", out.path().to_str().unwrap()));
    assert!(content.contains(snippet), "Generated output does not contain the expected content");
}

#[test]
fn generates_test_1_bms() {
    codegen_test_with_config("examples/bms/dbc/BMS.dbc", "examples/bms/bms.rs", vec![]);
}

#[test]
fn generates_test_2_bms() {
    codegen_test_with_config(
        "examples/bms/dbc/BMS.dbc",
        "examples/bms/bms_whitelist.rs",
        vec!["--whitelist", "641"],
    );
}

#[test]
fn generates_test_3_bms() {
    codegen_test_with_config(
        "examples/bms/dbc/BMS.dbc",
        "examples/bms/bms_blacklist.rs",
        vec!["--blacklist", "641"],
    );
}

#[test]
fn generates_test_1_model3() {
    codegen_test_with_config(
        "examples/model3/dbc/model3can.dbc",
        "examples/model3/model3can.rs",
        vec![],
    );
}

#[test]
fn generates_test_2_model3() {
    codegen_test_with_config(
        "examples/model3/dbc/model3can.dbc",
        "examples/model3/model3can_whitelist.rs",
        vec!["--whitelist", "257"],
    );
}

#[test]
fn generates_test_3_model3() {
    codegen_test_with_config(
        "examples/model3/dbc/model3can.dbc",
        "examples/model3/model3can_blacklist.rs",
        vec!["--blacklist", "257"],
    );
}

#[test]
fn generates_test_sections_not_in_order() {
    codegen_no_error("tests/dbc/not_in_order.dbc");
}

#[test]
fn test_variant_generation() {
    codegen_test_snippet("tests/dbc/val.dbc", r#"pub fn set_raw_value(&mut self, value: u8, data: &mut[u8]) {
            data.view_bits_mut::<Msb0>()[5..7].store_be(value);
        }

        pub fn set_as_def (&mut self, signal_def: DbcLengthWithCode, data: &mut[u8])-> Result<(),CanError> {
            match signal_def {
                DbcLengthWithCode::TooLong => self.set_raw_value(3, data),
                DbcLengthWithCode::_Other(x) => self.set_typed_value(x,data)
            }
        }
        fn get_typed_value(&self) -> f64 {
            self.value
        }

        fn set_typed_value(&mut self, value:f64, data:&mut [u8]) -> Result<(),CanError> {
            if value < 0_f64 || 1.5_f64 < value {
                return Err(CanError::new("invalid-signal-value",format!("value={} not in [0..1.5]",value)));
            }
            let factor = 0.5_f64;
            let offset = 0_f64;
            let value = ((value - offset) / factor) as u8;
            data.view_bits_mut::<Msb0>()[5..7].store_be(value);
            Ok(())
        }"#, vec![]);
}
