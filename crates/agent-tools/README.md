# agent-tools

CLI tool for managing Claude Code skills and settings.

## Installation

```bash
# Clone to ~/.agent-tools (or symlink)
git clone https://github.com/wthrk/agent-tools.git ~/.agent-tools
# Or: ln -s /path/to/agent-tools ~/.agent-tools

# Build and install
cd ~/.agent-tools
cargo make install

# Add to PATH
export PATH="$HOME/.agent-tools/bin:$PATH"
```

## Usage

```bash
# Initialize agent-tools
agent-tools init

# List available skills
agent-tools skill list

# Install a skill to current project
agent-tools skill install <name>

# Show installed skills
agent-tools skill installed

# Update a skill
agent-tools skill update <name>
agent-tools skill update --all

# Remove a skill
agent-tools skill remove <name>

# Show diff between installed and source
agent-tools skill diff <name>

# Link/unlink skills to ~/.claude/skills/
agent-tools link <name>
agent-tools unlink <name>

# Sync based on config.yaml
agent-tools sync

# Show status
agent-tools status

# Clean up old backups
agent-tools cleanup
```

See `agent-tools --help` for all options.

## Development

```bash
# Run tests in Docker
cargo make docker-test

# Build Docker test image
cargo make docker-test-build
```

## License

MIT
