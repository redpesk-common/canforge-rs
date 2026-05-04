use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;

// DBC minimal viable pour ton parser+generator.
const MIN_DBC: &str = r#"VERSION "1.0"
NS_ :
BU_: ECU
BO_ 1 MSG: 8 ECU
"#;

fn bin_path() -> std::path::PathBuf {
    assert_cmd::cargo::cargo_bin!("dbcparser-cli").to_path_buf()
}

#[test]
fn fails_when_missing_required_args() {
    // Clap doit refuser sans -i/-o (required=true)
    Command::new(bin_path())
        .assert()
        .failure()
        .stderr(predicate::str::is_match("(?i)usage:").unwrap())
        .stderr(predicate::str::contains("--in"))
        .stderr(predicate::str::contains("--out"));
}

#[test]
fn fails_when_input_file_missing() {
    Command::new(bin_path())
        .args(["-i", "does-not-exist.dbc", "-o", "out.rs"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("input file does not exist"));
}

#[test]
fn generates_with_default_header() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dbc = tmp.child("in.dbc");
    dbc.write_str(MIN_DBC).unwrap();

    let out = tmp.child("gen.rs");

    Command::new(bin_path())
        .args(["-i", dbc.path().to_str().unwrap(), "-o", out.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Generated:"));

    out.assert(predicate::path::exists());
    // Vérifie que l’en-tête par défaut est présent
    out.assert(predicate::str::contains("<- DBC file Rust mapping ->"));
}

#[test]
fn generates_without_header_when_no_header_flag() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dbc = tmp.child("in.dbc");
    dbc.write_str(MIN_DBC).unwrap();

    let out = tmp.child("gen.rs");

    Command::new(bin_path())
        .args([
            "-i",
            dbc.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--no-header",
        ])
        .assert()
        .success();

    out.assert(predicate::path::exists());
    out.assert(predicate::str::contains("<- DBC file Rust mapping ->").not());
}

#[test]
fn generates_with_custom_header_file() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dbc = tmp.child("in.dbc");
    dbc.write_str(MIN_DBC).unwrap();

    let header = tmp.child("header.txt");
    header.write_str("// MY-CUSTOM-HEADER\n").unwrap();

    let out = tmp.child("gen.rs");

    Command::new(bin_path())
        .args([
            "-i",
            dbc.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--header-file",
            header.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    out.assert(predicate::path::exists());
    out.assert(predicate::str::contains("MY-CUSTOM-HEADER"));
}

#[test]
fn accepts_whitelist_and_blacklist_hex_and_dec() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dbc = tmp.child("in.dbc");
    // Ajoute un 2e message pour traverser le chemin qui filtre
    let content = r#"VERSION "1.0"
NS_ :
BU_: ECU
BO_ 257 MSG_A: 8 ECU
BO_ 513 MSG_B: 8 ECU
"#;
    dbc.write_str(content).unwrap();

    let out = tmp.child("gen.rs");

    Command::new(bin_path())
        .args([
            "-i",
            dbc.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--whitelist",
            "0x101,0x201,513", // 0x201 et 513==0x201 -> doublons OK
            "--blacklist",
            "0x101", // retire 0x101 au final
        ])
        .assert()
        .success();

    out.assert(predicate::path::exists());
    // On ne sait pas facilement introspecter le contenu généré (structure),
    // mais si ça compile et produit un fichier c’est déjà un test utile.
}

#[test]
fn rejects_bad_whitelist_value() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dbc = tmp.child("in.dbc");
    dbc.write_str(MIN_DBC).unwrap();

    let out = tmp.child("gen.rs");

    Command::new(bin_path())
        .args([
            "-i",
            dbc.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--whitelist",
            "0xZZZ", // invalide
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid hex id"));
}

#[test]
fn rejects_duplicate_can_ids() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dbc = tmp.child("in.dbc");
    let content = r#"VERSION "1.0"
NS_ :
BU_: ECU
BO_ 100 MSG_A: 8 ECU
BO_ 100 MSG_B: 8 ECU
"#;
    dbc.write_str(content).unwrap();

    let out = tmp.child("gen.rs");

    Command::new(bin_path())
        .args(["-i", dbc.path().to_str().unwrap(), "-o", out.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("duplicate CAN id 100 / 0x64"));
}

#[test]
fn rejects_duplicate_generated_message_identifiers() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dbc = tmp.child("in.dbc");
    let content = r#"VERSION "1.0"
NS_ :
BU_: ECU
BO_ 100 FOO_BAR: 8 ECU
BO_ 101 FooBar: 8 ECU
"#;
    dbc.write_str(content).unwrap();

    let out = tmp.child("gen.rs");

    Command::new(bin_path())
        .args(["-i", dbc.path().to_str().unwrap(), "-o", out.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("duplicate generated message identifier 'FooBar'"));
}

#[test]
fn generates_bit_slice_length_guards() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dbc = tmp.child("in.dbc");
    let content = r#"VERSION "1.0"
NS_ :
BU_: ECU
BO_ 100 MSG: 2 ECU
 SG_ MySig : 8|8@1+ (1,0) [0|255] "" ECU
VAL_ 100 MySig 0 "Zero" 1 "One" ;
"#;
    dbc.write_str(content).unwrap();

    let out = tmp.child("gen.rs");

    Command::new(bin_path())
        .args(["-i", dbc.path().to_str().unwrap(), "-o", out.path().to_str().unwrap()])
        .assert()
        .success();

    let generated = fs::read_to_string(out.path()).unwrap();

    assert!(generated.contains("if frame.data.len() * 8 < 16 {"));
    assert!(generated.contains("return 0;"));
    assert!(generated.contains(
        "pub fn set_raw_value(&mut self, value: u8, data: &mut[u8]) -> Result<(),CanError>"
    ));
    assert!(generated.contains("if data.len() * 8 < 16 {"));
    assert!(generated.contains("\"invalid-buffer-length\""));
}

#[test]
fn generates_mux_length_guard() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dbc = tmp.child("in.dbc");
    let content = r#"VERSION "1.0"
NS_ :
BU_: ECU
BO_ 100 MSG: 2 ECU
 SG_ Mux M : 0|4@1+ (1,0) [0|15] "" ECU
 SG_ Payload m1 : 8|8@1+ (1,0) [0|255] "" ECU
"#;
    dbc.write_str(content).unwrap();

    let out = tmp.child("gen.rs");

    Command::new(bin_path())
        .args(["-i", dbc.path().to_str().unwrap(), "-o", out.path().to_str().unwrap()])
        .assert()
        .success();

    let generated = fs::read_to_string(out.path()).unwrap();

    assert!(generated.contains("if frame.data.len() * 8 < 4 {"));
    assert!(generated.contains("\"invalid-frame-length\""));
    assert!(generated.contains("frame too short for mux 'Mux'"));
}

#[test]
fn rejects_zero_signal_factor() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dbc = tmp.child("in.dbc");
    let content = r#"VERSION "1.0"
NS_ :
BU_: ECU
BO_ 100 MSG: 8 ECU
 SG_ MySig : 0|8@1+ (0,0) [0|255] "" ECU
"#;
    dbc.write_str(content).unwrap();

    let out = tmp.child("gen.rs");

    Command::new(bin_path())
        .args(["-i", dbc.path().to_str().unwrap(), "-o", out.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("signal:MySig has invalid factor 0"));
}

#[test]
fn generates_finite_float_value_guard() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let dbc = tmp.child("in.dbc");
    let content = r#"VERSION "1.0"
NS_ :
BU_: ECU
BO_ 100 MSG: 8 ECU
 SG_ MySig : 0|8@1+ (0.5,1) [0|255] "" ECU
"#;
    dbc.write_str(content).unwrap();

    let out = tmp.child("gen.rs");

    Command::new(bin_path())
        .args(["-i", dbc.path().to_str().unwrap(), "-o", out.path().to_str().unwrap()])
        .assert()
        .success();

    let generated = fs::read_to_string(out.path()).unwrap();

    assert!(generated.contains("if !value.is_finite()"));
    assert!(generated.contains("value must be finite"));
    assert!(generated.contains("if !__raw_f.is_finite()"));
    assert!(generated.contains("raw value must be finite"));
}
