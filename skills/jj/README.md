# jj - Jujutsu バージョン管理スキル

## 目次

- [重要: 実行ポリシー](#重要-実行ポリシー)
- [哲学: 並列開発がデフォルト](#哲学-並列開発がデフォルト)
- [並列実行モデル](#並列実行モデル)
- [クイックリファレンス](#クイックリファレンス)
- [コマンド](#コマンド)
- [高度な機能](#高度な機能)
- [エージェントプロトコル](#エージェントプロトコル)
- [主要概念](#主要概念)
- [一般的なワークフロー](#一般的なワークフロー)
- [エラー回復](#エラー回復)

## 重要: 実行ポリシー

**引数なしで呼び出された場合、Bashコマンドを実行してはならない。**

`/jj <subcommand>` が引数なしで呼び出された場合:
1. **Bashを呼び出さない** - jjコマンドの実行を試みない
2. **このファイルからドキュメントセクションを表示**
3. **コードブロック例を含める**
4. **ユーザーに何をしたいか確認**

引数付きで呼び出された場合（例: `/jj new main -m "task"`）:
- コマンドを実行して結果を報告

**読み取り専用コマンド**（`/jj status`, `/jj log`, `/jj diff`）:
- 現在の状態を表示するため即座に実行可

## 哲学: 並列開発がデフォルト

すべての作業は「多数の並列タスクの1つ」として扱われる（単一タスク = N=1の特殊ケース）。

- タスク = 独立したchange（`jj new main -m "task"`）
- タスク間はいつでも切り替え可能（`jj edit`）
- すべてがoperation logで取り消し可能
- jjは思考/履歴の並列性を解決（実行の並列性はworkspaceのみ）

## 並列実行モデル

| シナリオ | 方法 | ディレクトリ |
|----------|------|-------------|
| 単一エージェントがタスク切り替え | `jj edit` | 単一 |
| 複数エージェントが同時実行 | `jj workspace` | エージェントごとに分離 |

**workspaceが必要な理由:** 単一のworking copy = 単一のファイルシステム状態。複数ライター = 回復が必要な競合状態。

**定義:** 「同時」= 任意の並行ファイルシステムライター（エージェント、CI、ウォッチャー、ビルド）

## クイックリファレンス

| カテゴリ | コマンド |
|----------|----------|
| コア | new, describe, status, log, diff |
| ナビゲーション | edit（`@-`で前、`@+`で次も可） |
| 整理 | split, squash, abandon, restore |
| 同期 | bookmark, rebase, push, undo |

## コマンド

### /jj new - 新規Change作成

新しい空のchangeを作成。jjの基本操作。

```bash
jj new main -m "feat: implement feature X"  # 推奨
```

### /jj split - Change分割

**警告: デフォルトはインタラクティブ - ブロックする！**

```bash
# 推奨: --pathフラグで非インタラクティブ
jj split --path <file1> --path <file2>
```

### /jj push - リモートへプッシュ

**重要: プッシュにはBookmarkが必須。**

```bash
jj bookmark set feature-x
jj git push --bookmark feature-x
```

### workspace - 同時実行

複数の同時プロセス用に別々の作業ディレクトリを作成。

```bash
jj workspace add ../agent-a --rev main -m "Agent A task"
jj workspace add ../agent-b --rev main -m "Agent B task"
```

**必要な場合:**
- 複数のAIエージェントが並列で動作
- 同時ファイルシステムアクセスがあるシナリオ

**不要な場合:**
- 単一の開発者/エージェントがタスク間を切り替え（`jj edit`を使用）

## エージェントプロトコル

### ルール
- 編集前に必ずchange境界を宣言（`jj new`/`jj edit`）
- 1タスク = 1changeを維持
- operation logをセーフティネットとして使用
- `jj split`には`--path`を使用
- 同時実行エージェントには別々のworkspaceが必要（単一ディレクトリ = ファイル競合）

### 警告
- `--path`なしの`jj split`はブロックする
- `jj squash -i`（インタラクティブ）はブロックする
- スコープなしの`jj rebase`は意図しないchangeをリベースする可能性
- bookmarkなしのpushは失敗する
- 単一ディレクトリ内の複数エージェントはファイル混入を引き起こす → `jj workspace`を使用、または`jj abandon` + `jj new`で回復
- バックグラウンドで動作するCI/file watcherは競合リスクを増加させる可能性

## 主要概念

### Change vs Commit
- **Change**: 変更可能、change ID（`abc123`のような文字）で識別
- **Commit**: 不変のスナップショット、コミットハッシュで識別

### @ シンボル
- `@` = 現在のworking copy
- `@-` = 親
- `@+` = 子
- `@--` = 祖父母

### Operation Log
- すべてのjjコマンドが記録される
- `jj undo`で最後の操作を取り消し
- `jj op restore <id>`で任意の時点に復元

## 一般的なワークフロー

**クイックパターン:**
- 新規作業: `jj git fetch && jj new main -m "feat: X" && jj bookmark set X && jj git push --bookmark X`
- タスク切り替え: `jj edit <rev>`
- mainから更新: `jj git fetch && jj rebase -d main`

**詳細:** [workflow-basics.md](references/workflow-basics.md) | [workflow-pr.md](references/workflow-pr.md) | [workflow-concurrent.md](references/workflow-concurrent.md)

## エラー回復

- **任意のミスを取り消し:** `jj undo`
- **既知の良好な状態に復元:** `jj op restore <id>`

### 複数エージェント同時実行時のファイル混入からの回復
```bash
# 1. 操作履歴を確認
jj op log

# 2. まずクリーンな状態に戻す
jj undo  # または jj op restore <last-good-op-id>

# 3. 必要なら汚染ファイルを分離
jj diff && jj split --path <unwanted-file> && jj abandon

# 4. 最終手段: change全体を破棄
jj abandon <contaminated-change>
jj new main -m "Clean restart"
```
