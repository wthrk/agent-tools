# Skill Creator

## 概要

確立されたベストプラクティスに従ってClaude Codeスキルを作成・メンテナンスします。新規スキル作成、既存スキル修正、スキル構造の検証の3つのモードをサポートします。

## 使用条件

- 新しいスキルをゼロから作成する場合
- 既存のスキルを修正・改善する場合
- デプロイ前にスキルを検証する場合
- 検証エラーや警告を修正する場合

## ワークフロー

### モードA: 新規スキル作成

1. SKILL.md, README.md, AGENTS.mdを含むスキルディレクトリを作成
2. SKILL.mdを編集:
   - 三人称で説明を記述（"Creates...", "Analyzes..."）
   - "Use when"トリガー条件を含める
   - 説明は100-300文字に収める
3. README.mdを日本語に翻訳
4. モードCで検証
5. スキルをデプロイ

### モードB: 既存スキル修正

1. スキルディレクトリ内の全ファイルを読む
2. references/best-practices.mdでガイドラインを確認
3. ベストプラクティスに従って修正を適用
4. README.mdをSKILL.mdの変更と同期
5. モードCで検証

### モードC: スキル検証

1. スキルファイルの静的検証を実行
2. エラーがあれば続行前に修正
3. 全スキルファイルを読む（SKILL.md, README.md, references/*）
4. ベストプラクティスと照合:

**Description検証**
- 三人称で記述されているか（"Creates...", "Analyzes..."）
- "Use when"句を含むか
- 100-300文字の推奨範囲内か
- 禁止文字（`<` `>`）を含まないか

**構造検証**
- Overview, When to Use, The Process, Tipsセクションがあるか
- The Processに番号付きステップがあるか

**サイズ検証**
- SKILL.mdが500行未満か
- 5000語未満か

**参照検証**
- references/の深度が1レベル以内か
- 100行超のファイルに目次があるか

**禁止ファイル検証**
- CHANGELOG.md, INSTALLATION_GUIDE.md, QUICK_REFERENCE.mdがないか

**同期検証**
- README.mdがSKILL.mdと内容が一致しているか

5. 問題を報告（問題の説明、場所、修正案）
6. 要求があれば自動修正

## ヒント

- descriptionは三人称 + "Use when" + 100-300文字
- SKILL.mdは500行未満、5000語未満を維持
- 重要情報はファイル先頭に配置
- 100行超のファイルには目次を追加
- references/は1レベル深度のみ
- README.mdは常にSKILL.mdと同期
- 一人称禁止（"I can help..."はNG）
- 時間依存情報禁止（"2024年時点で..."はNG）

## チェックリスト

- [ ] description: 三人称 + "Use when" + 100-300文字 + `<>`なし
- [ ] name: `^[a-z0-9][a-z0-9-]*[a-z0-9]$`に一致、64文字以内
- [ ] SKILL.md: 500行未満、5000語未満
- [ ] 構造: Overview, When to Use, The Process, Tips
- [ ] references/: 1レベル深度のみ
- [ ] 100行超ファイル: 目次あり
- [ ] 禁止ファイル: なし
- [ ] README.md: SKILL.mdと同期
- [ ] AGENTS.md: 同期指示あり
