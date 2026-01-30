# skill-test

CLI tool for testing Claude Code skills.

## Installation

```bash
cargo install --path crates/skill-test
```

## Usage

```bash
# Test current directory as skill directory
skill-test

# Test specific skill directory
skill-test ./my-skill

# Test multiple skill directories
skill-test ./skills/a ./skills/b

# With options
skill-test --iterations 5 --threshold 80 --verbose
```

See `skill-test --help` for all options.

For detailed documentation, see [SKILL-TESTING.md](SKILL-TESTING.md).

## License

MIT
