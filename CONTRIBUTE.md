# Contribute

## improve your Rust code

Use **Clippy** and **rustfmt** to keep the codebase clean, idiomatic, and consistent.

### why clippy and rustfmt are mandatory in an industrial rust project

A professional codebase needs **predictability**, **safety**, and **speed of iteration**. Two tools make that baseline real:

- **rustfmt** enforces a single canonical style.
- **Clippy** enforces a baseline of code quality by flagging common mistakes and non-idiomatic patterns.

Together they reduce defects, shorten review time, and make the code easier to maintain over years and teams.

---

## rustfmt (formatting you can rely on)

### what it guarantees

- **Zero bikeshedding:** one style, produced automatically. Reviews focus on logic, not whitespace.
- **Stable diffs:** predictable formatting reduces noisy diffs and makes `git blame` more meaningful.
- **Onboarding made easy:** new contributors don’t need to learn a house style.
- **Tooling interoperability:** editors, CI, and pre-commit hooks can all run the same formatter.

### organization policy (recommended)

Formatting is required; pull requests must pass:

```bash
cargo fmt --all --check
```

A project-local `rustfmt.toml` defines only needed deviations (often: none).

If you use `rustup` (recommended):

```bash
rustup update
rustup component add rustfmt
```

### run fmt

```bash
cargo fmt --all
```

---

## clippy (linting that prevents subtle bugs)

### clippy:what it guarantees

- **Bug prevention:** catches suspicious code (`unwrap()` in tests ok, in prod not ok; needless clones; wrong iterator bounds; `Mutex` in async contexts; etc.).
- **Idiomatic Rust:** nudges toward patterns the ecosystem expects, improving readability and performance.
- **Security hygiene:** warns about `panic!` in FFI, `mem::uninitialized`, or non-`Send`/`Sync` in shared contexts, among many others.

### clippy:organization policy (recommended)

Clippy runs on every commit and in CI, with warnings elevated to errors.

If you use `rustup` (recommended):

```bash
rustup update
rustup component add clippy
```

### run clippy

```bash
cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic
```

- `--all-targets` lints libs, bins, tests, benches, examples
- `--all-features` checks all feature combinations
- `-D warnings` fails the build on any warning
- `-W clippy::pedantic` enables extra strict lints

---

## documentation (keep docs first-class)

Documentation is part of the public API and must always compile **without warnings**.

### documentation:what it guarantees

- **Discoverability:** users and contributors can understand APIs directly from `cargo doc`.
- **Quality assurance:** broken links or outdated examples are caught early.
- **Continuous integration:** docs are verified just like code.

### documentation:organization policy (recommended)

1. Every public item (`pub`, `pub(crate)`) must have a Rustdoc comment (`///` or `//!`).
2. Code examples in documentation must compile successfully (doctests are enabled by default).
3. Documentation builds must pass without warnings.

### build and check documentation

To build the documentation:

```bash
cargo doc --no-deps --workspace --all-features
```

To enforce no warnings:

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

To run doctests:

```bash
cargo test --doc
```

If you contribute new public APIs, update the documentation accordingly.

---

## quality gates (must pass before merge)

1. **Formatting:** `cargo fmt --all --check`
2. **Linting:** `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. **Testing:** `cargo test --workspace --all-features`
4. **Security:** `cargo audit` (no unpatched advisories) and `cargo deny check` (license/dependency policy)
5. **Documentation:** `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` (no doc warnings)

## pre-commit (automated local checks)

We use **pre-commit** to run fast, consistent checks locally before code reaches CI.
This reduces churn in reviews and keeps the repo clean.

### policy (recommended)

- Pre-commit hooks must pass locally before opening/merging a PR.
- Run `pre-commit install` once per clone; hooks will then run automatically on staged files.
- CI will run `pre-commit` across the repo (`pre-commit run --all-files`) to enforce the same rules.

### install

```bash
# Using pipx (recommended)
pipx install pre-commit

# Or via pip
pip install --user pre-commit

# Enable hooks in this repo
pre-commit install

# (Optional) Run on the whole repo once
pre-commit run --all-files
```

### what runs in hooks

- **Formatting**: `cargo fmt --all --check`
- **Linting**: `cargo clippy --all-targets --all-features -- -D warnings`
- **YAML hygiene**: validate YAML/TOML; trim whitespace; ensure newline at EOF
- **Commit hygiene**: block large binaries, enforce file size limits

> Note: clippy and fmt hooks use **local hooks** so they run with your pinned toolchain (via `rustup` and `rust-toolchain.toml`).

---

### updates to other sections

Add this to your **quality gates** section:

- **Pre-commit:** `pre-commit run --all-files` (must pass)

Add this to your **PR checklist**:

- [ ] Pre-commit hooks installed and passing locally (`pre-commit run --all-files`)
