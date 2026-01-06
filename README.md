# canforge-rs: `dbcparser`, `dbcparser-cli`, and `parse-dbc`

A pragmatic DBC (CAN database) parser and Rust code generator.

This repository is a Rust workspace that currently provides:

- **`dbcparser`**: library that parses DBC files and exposes a domain model used for code generation.
- **`dbcparser-cli`**: command-line tool that generates Rust code from a DBC file, with filtering and configuration support.
- **`parse-dbc`** (in `dbcparser-check-cli/`): a small helper CLI used to run the generator against a DBC file and a candump log (primarily for checks/demos).

> Status: used with real DBC examples, still evolving. Public API and CLI options may change.

---

## Project layout

This matches the current repository:

```text
.
├── Cargo.toml              # workspace
├── Cargo.lock
├── README.md
├── spec-grammar.md         # DBC grammar notes
├── CONTRIBUTE.md
├── deny.toml
├── rustfmt.toml
├── .pre-commit-config.yaml
├── dbcparser/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs          # lib root (re-exports parser/data/gencode)
│   │   ├── parser.rs       # DBC parser (nom-based)
│   │   ├── data.rs         # domain types (messages, signals, etc.)
│   │   └── gencode.rs      # Rust code generation
│   └── tests/
│       └── test.rs         # integration tests for parser & lib
├── dbcparser-cli/
│   ├── Cargo.toml
│   ├── src/
│   │   └── main.rs         # CLI: generate Rust from DBC
│   ├── tests/
│   │   ├── cli.rs          # CLI behaviour tests
│   │   ├── test_bms.rs     # BMS example tests
│   │   └── test_model3.rs  # Model3 example tests
│   └── examples/
│       ├── bms/
│       │   ├── bms.rs
│       │   ├── bms_blacklist.rs
│       │   └── dbc/
│       │       └── BMS.dbc
│       └── model3/
│           └── dbc/
│               └── model3can.dbc
└── dbcparser-check-cli/
    ├── Cargo.toml
    └── src/
        └── parse-dbc.rs    # bin: parse-dbc
```

---

## Features

### Current (implemented)

Library (`dbcparser`):

- DBC parsing into an internal domain model (messages, signals, attributes, etc.).
- Domain types and helpers in `src/data.rs`.
- Code generator in `src/gencode.rs` that turns a DBC into Rust modules and types.

CLI (`dbcparser-cli`):

- Generate Rust code from a DBC file:
  - optional whitelist/blacklist of CAN IDs,
  - optional header injection (custom file) or header removal,
  - configuration via YAML file,
  - ability to save the *effective* configuration to YAML for later reuse,
  - verbose mode to print the effective configuration as YAML.

Helper CLI (`parse-dbc`):

- Runs the generator with a DBC file and an output path (typically used with a candump log workflow).
- Intended mainly as a lightweight check/demo tool.

Real-world examples:

- BMS DBC: `dbcparser-cli/examples/bms/dbc/BMS.dbc`
- Model3 DBC: `dbcparser-cli/examples/model3/dbc/model3can.dbc`
- Example code and/or generated Rust files live under `dbcparser-cli/examples/`.

### Design goals / roadmap

These are **not fully implemented yet**, but drive the design:

- Clear pipeline: `lexer (&[u8]) → parser (AST) → validator → IR (domain model) → generators`.
- Precise error model with spans (line/column) and categories, using `thiserror` in the lib and a rich reporter (`color-eyre`/`miette`) in the CLI.
- Additional CLI subcommands (besides code generation): `parse`, `validate`, `dump`, `convert`, `query`, `stats`, `grep-db`.
- Optional converters to JSON/YAML/TOML via `serde`.
- Strong validation rules: overlapping signals, multiplexing, ID uniqueness, value ranges, etc.
- Fuzzing and benchmarks for robustness and performance.

---

## Build from source

### Requirements

- Rust stable (edition 2021)

Optional developer tools:

- `pre-commit` (recommended if you want local hooks)
- `cargo-deny` (to use `deny.toml`)

### Clone and build

```bash
git clone https://github.com/redpesk/canforge-rs.git
cd canforge-rs
cargo build
```

Build only the CLI:

```bash
cargo build -p dbcparser-cli
```

Build only the helper CLI:

```bash
cargo build -p parse-dbc
```

---

## CLI usage

### `dbcparser-cli`

The main binary is `dbcparser-cli`.
It currently focuses on **generating Rust code from a DBC file**.

```text
Usage: dbcparser-cli [OPTIONS]

Options:
  -i, --in <INFILE>                Input DBC file (required unless a YAML config is provided)
  -o, --out <OUTFILE>              Output Rust file path (required unless a YAML config is provided)
      --uid <UID>                  Optional UID (module/namespace root in generated code) [default: DbcSimple]
      --header-file <HEADER_FILE>  Header text file to prepend (overrides built-in header if provided)
      --no-header                  Disable default header completely
      --whitelist <WHITELIST>      Whitelist CAN IDs (CSV, hex 0xABC or decimal): e.g. "0x101,0x121,201"
      --blacklist <BLACKLIST>      Blacklist CAN IDs (CSV, hex 0xABC or decimal): e.g. "0x101,0x121,201"
      --config <YAML>              Load parameters from a YAML configuration file
      --save-config <YAML>         Save the effective parameters to this YAML file
  -v, --verbose                    Verbose mode: print effective configuration as YAML
  -h, --help                       Print help
  -V, --version                    Print version
```

Run CLI help:

```bash
cargo run -p dbcparser-cli -- --help
```

#### ID list formats

Whitelist and blacklist options accept:

- hexadecimal with or without `0x`,
- decimal integers.

Examples:

```bash
--whitelist "0x101,0x121,201"
--blacklist "0x101,0x200"
```

#### Basic examples

Generate Rust code with defaults:

```bash
cargo run -p dbcparser-cli -- \
  --in dbcparser-cli/examples/bms/dbc/BMS.dbc \
  --out ./generated_bms.rs
```

No header:

```bash
cargo run -p dbcparser-cli -- \
  --in dbcparser-cli/examples/model3/dbc/model3can.dbc \
  --out ./generated_model3.rs \
  --no-header
```

Custom header file:

```bash
cargo run -p dbcparser-cli -- \
  --in dbcparser-cli/examples/bms/dbc/BMS.dbc \
  --out ./generated_bms.rs \
  --header-file ./HEADER.txt
```

Whitelist and blacklist filtering:

```bash
cargo run -p dbcparser-cli -- \
  --in dbcparser-cli/examples/model3/dbc/model3can.dbc \
  --out ./generated_model3_filtered.rs \
  --whitelist "0x101,0x201,513" \
  --blacklist "0x101"
```

#### YAML configuration

Load parameters from a YAML file:

```bash
cargo run -p dbcparser-cli -- --config ./config.yaml
```

Save the effective configuration (after CLI parsing) to a YAML file:

```bash
cargo run -p dbcparser-cli -- \
  --in dbcparser-cli/examples/bms/dbc/BMS.dbc \
  --out ./generated_bms.rs \
  --whitelist "0x101,0x121" \
  --save-config ./effective.yaml
```

Verbose mode prints the effective configuration as YAML to stdout:

```bash
cargo run -p dbcparser-cli -- --config ./config.yaml --verbose
```

### `parse-dbc` (helper CLI)

The helper binary is `parse-dbc` (package: `parse-dbc`).

```bash
cargo run -p parse-dbc -- <DBC_FILE> <OUTPUT_FILE>
```

The tool is intentionally minimal and is mainly used for checks/demos. For production-style code generation, prefer `dbcparser-cli`.

---

## Library usage (high-level)

The library is available as the `dbcparser` crate within this workspace.

Current internal structure:

- `src/parser.rs` — parsing logic (nom-based).
- `src/data.rs` — data structures representing messages, signals, attributes, etc.
- `src/gencode.rs` — code generator using the parsed DBC representation.
- `src/lib.rs` — crate root, re-exporting the main types and functions.

API details are still evolving; expect breaking changes while the internal design converges toward:

- `Dbc::from_str(&str) -> Result<Dbc, DbcError>`
- `Dbc::from_reader<R: Read>(R) -> Result<Dbc, DbcError>`
- iterators over messages/signals, explicit validation, and optional `serde` support.

---

## Testing

Run all tests:

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

The repository includes:

- integration tests for the library in `dbcparser/tests/test.rs`,
- CLI integration tests in `dbcparser-cli/tests/` using real DBC files (BMS, Model3).

---

## Design notes

High-level design direction:

- **Parsing**: `parser.rs` uses `nom` to read DBC files into an internal representation.
- **Domain model**: `data.rs` holds message/signal/attribute structures used by the generator.
- **Code generation**: `gencode.rs` transforms the parsed DBC into Rust modules and types.
- **Future pipeline** (roadmap):
  `lexer (&[u8]) → parser (AST) → validator → IR (domain model) → generators`.

Validation goals include:

- consistent CAN IDs,
- non-overlapping bit ranges for signals,
- consistent multiplexing,
- sensible value ranges and units.

---

## Roadmap

Planned improvements (subject to change):

- [ ] Extend grammar coverage for all DBC constructs (attributes, env vars, etc.).
- [ ] Introduce a lexer/AST/IR/validator pipeline for clearer separation of concerns.
- [ ] Introduce CLI subcommands:
  - `parse`: syntax-only parsing with diagnostics,
  - `validate`: semantic checks and return codes,
  - `dump`: structured listing,
  - `stats`: statistics on messages/signals/bit usage,
  - `grep-db`: search messages/signals by name/attributes.
- [ ] Add more real-world DBC fixtures for integration tests.
- [ ] Set up benchmarks and fuzzing for robustness and performance.

---

## License

This project is licensed under the **MIT License**.
See `LICENSE.MIT`.

---

## Acknowledgements

- Inspired by existing open-source DBC parsers and tools.
- Built using the Rust ecosystem, in particular `nom` for parsing.
- Thanks to contributors and users providing real-world DBC samples.
