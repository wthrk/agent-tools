# skill-tools

CLI tool for managing Claude Code skills and settings.

## Installation

```bash
# Clone to ~/.skill-tools (or symlink)
git clone https://github.com/wthrk/agent-tools.git ~/.skill-tools
# Or: ln -s /path/to/agent-tools ~/.skill-tools

# Build and install
cd ~/.skill-tools
cargo make install

# Add to PATH
export PATH="$HOME/.skill-tools/bin:$PATH"
```

## Usage

```bash
# Initialize skill-tools
skill-tools init

# List available skills
skill-tools skill list

# Install a skill to current project
skill-tools skill install <name>

# Show installed skills
skill-tools skill installed

# Update a skill
skill-tools skill update <name>
skill-tools skill update --all

# Remove a skill
skill-tools skill remove <name>

# Show diff between installed and source
skill-tools skill diff <name>

# Link/unlink skills to ~/.claude/skills/
skill-tools link <name>
skill-tools unlink <name>

# Sync based on config.yaml
skill-tools sync

# Show status
skill-tools status

# Clean up old backups
skill-tools cleanup
```

See `skill-tools --help` for all options.

## Development

```bash
# Run tests in Docker
cargo make docker-test

# Build Docker test image
cargo make docker-test-build
```

## License

MIT
