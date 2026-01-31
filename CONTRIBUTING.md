# Contributing

## Branch Strategy

- `main` への直接 push 禁止
- 全変更は PR → squash merge
- PR は 300 行以下推奨、1 週間以上のブランチ禁止

## Development with jj

```bash
# 1. main を最新に
jj git fetch
jj bookmark set main --to origin/main

# 2. 作業開始
jj new main
jj describe -m "feat(agent-tools): add feature"

# 3. PR 作成
jj bookmark create feature/xxx
jj git push --bookmark feature/xxx
# GitHub で PR 作成、CI 通過後 "Squash and merge"

# 4. クリーンアップ
jj git fetch && jj bookmark set main --to origin/main
jj bookmark delete feature/xxx
```

## Commit Convention

[Conventional Commits](https://conventionalcommits.org/) 形式。**PR タイトル**がこの形式である必要がある。

| Type | バージョン変化 |
|------|----------------|
| `feat` | minor |
| `fix` | patch |
| `feat!` / `fix!` | major |
| `docs`, `chore`, `refactor`, `test`, `ci` | なし |

**scope**: `agent-tools`, `skill-test`, `skill-test-core`, `xtask`, `workspace`, `ci`, `deps`

## Release

[release-plz](https://release-plz.dev/) による自動リリース。

1. `feat:` / `fix:` PR をマージ → リリース PR 自動作成
2. リリース PR をマージ → タグ + Release 作成
3. バイナリが自動ビルドされ Release に添付
