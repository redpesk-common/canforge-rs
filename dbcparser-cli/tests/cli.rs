use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
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
