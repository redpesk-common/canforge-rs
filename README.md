# dbcparser & dbcparser-cli

A fast, modular DBC (CAN database) parser and CLI toolkit written in Rust.

- **dbcparser**: library that tokenizes, parses, validates, and lowers DBC files into a clean IR (domain model).
- **dbcparser-cli**: command-line utility built on top of the library (parse, validate, generate Rust mappings, etc.).

> Status: early but test-driven. Public API may evolve.

---

## Features

- Lexer over `&[u8]` with line/column/span tracking.
- Grammar parser that produces a minimal **AST**.
- Validation pass that enforces structural and semantic rules.
- IR (domain model): `Dbc`, `Message`, `Signal`, `Attribute`, etc.
- Error model with precise spans and categories (built with `thiserror`).
- CLI (`dbcparser-cli`) to:
  - parse/validate a DBC file and print diagnostics,
  - dump or convert into other formats (JSON, YAML, TOML – planned),
  - query or filter messages/signals,
  - compute simple stats,
  - generate small Rust mapping stubs.

---

## Build from source

### Requirements

- **Rust** 1.70+ (stable)
- Optional developer tools:
  - `cargo-edit`
  - `cargo-criterion`
  - `cargo-fuzz` (requires nightly + LLVM tools)

### Clone and build

```bash
git clone https://github.com/redpesk/canforge-rs.git
cd canforge-rs
cargo build
```

To build the CLI only:

```bash
cargo build -p dbcparser-cli
```

Run the CLI help:

```bash
cargo run -p dbcparser-cli -- --help
```

---

## CLI usage

The CLI binary name is `dbcparser-cli` (you can rename via `[ [bin] ]` if desired).

```text
Usage: dbcparser-cli [OPTIONS] --in <INFILE> --out <OUTFILE>

Options:
  -i, --in <INFILE>          Input DBC file (required)
  -o, --out <OUTFILE>        Output Rust/print file (required for generator/dumps)
      --uid <UID>            Optional UID/root module name [default: DbcSimple]
      --header-file <PATH>   Prepend a custom header text (overrides default)
      --no-header            Disable header emission
      --whitelist <LIST>     Allow only these CAN IDs (CSV: 0xABC,201,513)
      --blacklist <LIST>     Exclude these CAN IDs (CSV)
  -h, --help                 Print help
  -V, --version              Print version
```

### ID list formats

- Accepts hex with or without `0x` and decimal:
  - `--whitelist "0x101,0x201,513"`
  - `--blacklist "0x101,0x200"`

### Examples

Generate a Rust mapping with defaults:

```bash
dbcparser-cli \
  -i ./dbcparser/examples/demo.dbc \
  -o ./generated.rs
```

No header:

```bash
dbcparser-cli -i in.dbc -o out.rs --no-header
```

Custom header file:

```bash
dbcparser-cli -i in.dbc -o out.rs --header-file ./HEADER.txt
```

Disable serde support in generated code:

```bash
dbcparser-cli -i in.dbc -o out.rs --serde-json false
```

Whitelist and blacklist filtering:

```bash
dbcparser-cli -i in.dbc -o out.rs \
  --whitelist "0x101,0x201,513" \
  --blacklist "0x101"
```

---

## Example DBC

A minimal DBC file the parser can handle:

```text
VERSION "1.0"
NS_ :
BU_: ECU
BO_ 1 MSG: 8 ECU
```

Convert it (if converter implemented):

```bash
dbcparser-cli convert -i in.dbc --to json
```

---

## Testing

Run unit and integration tests:

```bash
cargo test
```

Test only the library:

```bash
cargo test -p dbcparser
```

Test only the CLI:

```bash
cargo test -p dbcparser-cli
```

CLI integration tests use `assert_cmd`, `assert_fs`, and `predicates` to verify:

- missing or invalid argument handling,
- header toggles,
- whitelist/blacklist parsing,
- serde flag control.

---

## Benchmarks and fuzzing

Criterion benchmarks (if present):

```bash
cargo bench -p dbcparser
```

Fuzzing (if configured):

```bash
# install cargo-fuzz
cargo fuzz run parse_dbc
```

---

## Design notes

- **Pipeline**: `lexer → parser (AST) → validator → IR`
- **Parser**: purely syntactic; no business logic.
- **Validator**: checks semantic consistency (IDs, nodes, value ranges).
- **IR**: domain representation independent of DBC syntax.
- **Error model**: `DbcError` includes line/column span and category.
- **Extensible**: easy to add new DBC clauses and mappings.

---

## Roadmap

- [ ] Extend grammar coverage for all DBC constructs.
- [ ] JSON/YAML/TOML converters via optional `serde` feature.
- [ ] Rich `query` subcommand (filter by ID/name/pattern).
- [ ] Advanced `stats` command (bit usage, gaps, endianness).
- [ ] More real-world DBC fixtures for integration tests.
- [ ] Performance benchmarks on large DBC files.

---

## Contributing

Contributions and pull requests are welcome!
Please make sure to run:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features
cargo test --workspace
```

When adding or modifying grammar rules, include:

- unit tests near the parser function,
- integration tests in `dbcparser/tests/`,
- documentation updates in `spec-grammar.md`.

---

## License

This project is licensed under the **MIT License**.
See the `LICENSE` file for details.

---

## Acknowledgements

- Based on ideas from open DBC parsers and the `nom` parser combinator library.
- Thanks to contributors and testers providing real-world DBC samples.
