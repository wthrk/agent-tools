Language: Rust (edition 2024), workspace linting configured in Cargo.toml.

Style/conventions:
- rustfmt settings in skill-test/rustfmt.toml: max_width=100, use_field_init_shorthand=true, use_try_shorthand=true.
- Clippy lints at workspace level: all/pedantic/nursery/cargo warn; unwrap_used/expect_used/panic denied.
- Rust lints: unsafe_code denied.