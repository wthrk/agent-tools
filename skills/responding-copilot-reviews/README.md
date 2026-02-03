# Copilotレビュー対応スキル

## 概要

PRに対するGitHub Copilotの自動コードレビューコメントに対応するためのワークフロー。重要な原則は**批判的評価** - Copilotの提案がすべて正しいわけではない。

## 使用場面

- PRにCopilotレビューコメントがある時
- 自動レビューフィードバックを体系的に評価したい時
- PR上の複数のレビューコメントに対応する必要がある時

## 使い方

```
/responding-copilot-reviews [PR番号]
```

## プロセス

### ステップ1: Copilotコメント取得

PRからレビューコメントを取得（ページネーション付き、Copilotのみフィルタ）：

```bash
gh api --paginate repos/{owner}/{repo}/pulls/{pr}/comments \
  | jq '[.[] | select(.in_reply_to_id == null)]'
```

**フィルタ条件:**
- `in_reply_to_id == null` - 既に返信があるコメントはスキップ（重複返信を避ける）

**各コメントから必要なフィールド:**
- `id` - コメントID（ステップ5の返信に必要）
- `path` - ファイルパス
- `line` / `original_line` - 行番号（outdatedコメントではnullの場合あり）
- `diff_hunk` - コードコンテキスト（lineがnullの場合のフォールバック）
- `body` - コメント内容

### ステップ2: 各コメントにサブエージェントを起動

各コメントに対してTask toolでサブエージェントを並列起動：

```
Task tool (subagent_type: "general-purpose", run_in_background: true)
prompt: |
  このCopilotレビューコメントを評価してください：

  COMMENT_ID: {id}
  File: {path}
  Line: {line}（nullの場合は下のdiff_hunkを使用）
  Diff Hunk: {diff_hunk}
  Comment: {body}

  1. {path}の実際のコードを読む
     - lineが有効な場合：その行の周辺を読む
     - lineがnullの場合：diff_hunkでコードコンテキストを特定
  2. 提案が正しいか検証する
  3. Codexでセカンドオピニオンを取得：
     codex exec "Is this suggestion valid? [comment]. Context: [code snippet]"
  4. COMMENT_IDを含めて判定を返す（返信に必須）
```

**重要:**
- すべてのサブエージェントを並列で起動（複数のTask呼び出しを1メッセージで）
- レート制限を避けるため5-10並列に制限
- 各サブエージェントは出力にCOMMENT_IDを含めること

### ステップ3: 結果を収集し判定を決定

すべてのサブエージェントが完了したら、**COMMENT_IDを保持して**判定表を作成：

| COMMENT_ID | ファイル | サブエージェント判定 | 決定 |
|------------|----------|----------------------|------|
| 123456 | src/main.rs | ACCEPT - 有効な提案 | ✅ Accept |
| 123457 | src/lib.rs | REJECT - 誤検知 | ❌ Reject |
| 123458 | src/utils.rs | ACCEPT - 可読性向上 | ✅ Accept |

**エラーハンドリング:** サブエージェントが失敗またはタイムアウトした場合：
- そのコメントの評価を手動で再試行
- またはNEEDS_REVIEWとしてマークし同期的に処理

### ステップ4: 修正は個別コミット

受け入れた提案はコミットで修正：

```bash
jj new -m "style: address review comment - [説明]"
# 修正を適用
jj git push
```

**コミット戦略:**
- **デフォルト:** 修正ごとに1コミット（リバートしやすい、履歴が明確）
- **例外:** 同じ種類の修正は1コミットにまとめる（例：複数の「戻り値の型を追加」修正）
- **禁止:** 修正を元のコミットに混ぜない

### ステップ5: 各レビューコメントに直接返信

レビューコメントに直接返信（一般的なPRコメントではなく）：

**受け入れた提案の場合:**
```bash
gh api repos/{owner}/{repo}/pulls/{pr}/comments/{id}/replies \
  -f body="✅ Fixed in commit {hash}"
```

**却下した提案の場合:**
```bash
gh api repos/{owner}/{repo}/pulls/{pr}/comments/{id}/replies \
  -f body="❌ False positive. [理由]"
```

**ルール: issueコメントではなく、レビューコメントに直接返信する。**

## サブエージェントタスク

各サブエージェントは1つのコメントを評価し、以下を行う：

1. **コードを読む** - 実際のファイルを取得し、参照行を特定
   - `line`がnullまたはoutdatedの場合、`diff_hunk`でコードコンテキストを見つける
   - `diff_hunk`のユニークな文字列をファイル内で検索
2. **コンテキストを理解** - 周辺コードを読んで意図を把握
3. **提案を検証** - Copilotの提案が技術的に正しいか？
4. **Codexに相談** - `codex exec`で外部意見を取得
5. **判定を返す** - ACCEPTまたはREJECT（明確な理由付き、**COMMENT_ID必須**）

**サブエージェント出力形式:**
```
VERDICT: ACCEPT | REJECT
COMMENT_ID: {id}
FILE: {path}
REASONING: {説明}
FIX_SUGGESTION: {ACCEPTの場合、修正内容を記述}
```

**なぜサブエージェント？**
- 並列処理でレビューが高速化
- 各コメントに集中した評価
- 独立したCodex相談でバイアス防止
- コメントごとに明確な責任

## アンチパターン

| アンチパターン | 問題点 | 正しいアプローチ |
|----------------|--------|------------------|
| すべての提案を無批判に受け入れる | 誤検知で時間を浪費、間違った修正でコードが悪化 | 各提案を批判的に検証 |
| 一般コメントで返信する | レビュースレッドが壊れる、追跡困難 | レビューコメントに直接返信 |
| 修正を元のコミットに混ぜる | コミット履歴が汚染、リバートが困難 | 修正ごとに個別コミット |
| 議論せずに独断で判断する | 有効な反論を見逃す可能性 | サブエージェントがCodexと議論 |

## ヒント

- **検証をスキップしない**: ステップ2（サブエージェント評価）とステップ3（判定表）は必須 - 批判的評価が核心原則
- **outdatedコメントの対応**: `line`がnullの場合、`diff_hunk`または`original_line`でコンテキストを特定
- **却下理由を明確に記録**: 将来のレビュアー（人間またはAI）が明確な理由から恩恵を受ける
- **誤検知パターンを確認**: 一部のCopilot提案はコードベースに対して一貫して間違っている
- **`gh pr comment`より`gh api`を使用**: 直接APIはコメントスレッドをより細かく制御可能
- **一般的な誤検知を追跡**: チームのためにドキュメント化を検討
- **COMMENT_IDを全工程で保持**: IDがないとステップ5で正しいコメントに返信できない
