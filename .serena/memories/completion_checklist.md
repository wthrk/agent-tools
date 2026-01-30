When finishing a change:
- Run fmt check: `cargo fmt --all --check`
- Run type check: `cargo check --workspace --all-targets`
- Run clippy: `cargo clippy --workspace --all-targets -- -D warnings`
- Run tests: `cargo test --workspace`
- Run cargo-deny if dependency changes: `cargo deny check`