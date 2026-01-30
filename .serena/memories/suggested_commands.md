Common commands (from Makefile.toml and docs):
- Format check: `cargo fmt --all --check`
- Type check: `cargo check --workspace --all-targets`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Dependency/license check: `cargo deny check`
- Tests: `cargo test --workspace`
- CI task via cargo-make (if installed): `cargo make ci`

CLI usage:
- Install CLI: `cargo install --path crates/skill-test`
- Run: `skill-test --config tests/cases.yaml --contracts contracts/`