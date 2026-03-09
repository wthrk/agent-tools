# Codex RunPod Profile Template

This directory is a template root for the `runpod` Codex profile.

Place profile-specific files here (for example `config.toml`, `agents/`, or
other Codex settings) that should be copied into the runtime profile when
running:

```bash
agent-tools use runpod
```

Expected behavior:
- Files under `templates/codex/runpod/` are used as the source template.
- Runtime state is created under `~/.agent-tools/.local/profiles/runpod/codex`.
- `~/.codex` is switched to point at that runtime profile directory.

Keep this directory free of machine-local secrets. Use local overlays for
environment-specific values.
