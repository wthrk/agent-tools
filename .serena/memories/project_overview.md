Purpose: Rust workspace for a CLI tool that tests Claude Code skills (skill-test) plus a core library (skill-test-core). Repository includes documentation on skill development tips and skill testing.

Structure:
- README.md and TIPS.md: docs for skill development and testing.
- skill-test/: CLI/tooling docs and hooks; Rust crates under skill-test/crates/skill-test and skill-test/crates/skill-test-core.
- Makefile.toml: cargo-make tasks for fmt/check/clippy/deny/test/ci.

Entry points:
- skill-test CLI (install via `cargo install --path crates/skill-test`).