#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! serde_json = "1.0"
//! chrono = "0.4"
//! clap = { version = "4.0", features = ["derive"] }
//! serde = { version = "1.0", features = ["derive"] }
//! ```

use std::fs::{self, OpenOptions};
use std::io::{self, Write as IoWrite, Read};
use std::path::PathBuf;
use serde_json::json;
use chrono::Local;
use clap::Parser;
use serde::Deserialize;

#[derive(Parser)]
#[command(author, version, about = "CLAUDE.mdをClaude Code フックで読み込むスクリプト", long_about = None)]
struct Args {
    /// デバッグモードを有効化（hook-debug.logにログ出力）
    #[arg(short, long)]
    debug: bool,

    /// フックイベントを指定（UserPromptSubmit, PreToolUse, PostToolUse）
    #[arg(long)]
    hook_event: Option<String>,
}

#[derive(Deserialize, Debug)]
struct HookInput {
    session_id: Option<String>,
    transcript_path: Option<String>,
    cwd: Option<String>,
    hook_event_name: Option<String>,
    prompt: Option<String>,
    tool_name: Option<String>,
    tool_input: Option<serde_json::Value>,
}

fn log(message: &str, debug_mode: bool) {
    if !debug_mode {
        return;
    }

    let timestamp = Local::now().format("[%Y-%m-%d %H:%M:%S]");
    let log_path = format!("{}/.claude/hook-debug.log", std::env::var("HOME").unwrap());
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        if message.is_empty() {
            let _ = writeln!(file);
        } else {
            let _ = writeln!(file, "{}  {}", timestamp, message);
        }
    }
}

fn main() {
    let args = Args::parse();
    let debug_mode = args.debug;

    log("", debug_mode);
    log("================================================================================", debug_mode);
    log("=== フック開始 ===", debug_mode);
    log("================================================================================", debug_mode);

    // stdinから入力を読み取る
    log("", debug_mode);
    log("【受信情報】", debug_mode);

    let mut stdin_input = String::new();
    let hook_input: Option<HookInput> = match io::stdin().read_to_string(&mut stdin_input) {
        Ok(_) => {
            if !stdin_input.is_empty() {
                // JSONとしてパース
                match serde_json::from_str::<HookInput>(&stdin_input) {
                    Ok(parsed) => {
                        log(&format!("session_id      : {:?}", parsed.session_id.as_deref().unwrap_or("(none)")), debug_mode);
                        log(&format!("transcript_path : {:?}", parsed.transcript_path.as_deref().unwrap_or("(none)")), debug_mode);
                        log(&format!("cwd             : {:?}", parsed.cwd.as_deref().unwrap_or("(none)")), debug_mode);
                        log(&format!("hook_event_name : {:?}", parsed.hook_event_name.as_deref().unwrap_or("(none)")), debug_mode);

                        if let Some(ref tool_name) = parsed.tool_name {
                            log(&format!("tool_name       : {:?}", tool_name), debug_mode);
                        }

                        log("", debug_mode);

                        if let Some(ref prompt) = parsed.prompt {
                            log("prompt:", debug_mode);
                            for line in prompt.lines() {
                                log(&format!("  {}", line), debug_mode);
                            }
                        }

                        if let Some(ref tool_input) = parsed.tool_input {
                            log("tool_input:", debug_mode);
                            let tool_input_str = serde_json::to_string_pretty(tool_input).unwrap_or_else(|_| "(invalid json)".to_string());
                            for line in tool_input_str.lines() {
                                log(&format!("  {}", line), debug_mode);
                            }
                        }
                        Some(parsed)
                    }
                    Err(e) => {
                        log(&format!("stdin JSONパースエラー: {}", e), debug_mode);
                        None
                    }
                }
            } else {
                log("stdin: (空)", debug_mode);
                None
            }
        }
        Err(e) => {
            log(&format!("stdin読み取りエラー: {}", e), debug_mode);
            None
        }
    };

    // 重要な環境変数のみ
    log("", debug_mode);
    log("【環境変数（主要）】", debug_mode);
    let important_vars = [
        "CLAUDE_PROJECT_DIR",
        "CLAUDE_CODE_ENTRYPOINT",
        "HOME",
        "USER",
        "PWD"
    ];
    for var in &important_vars {
        if let Ok(value) = std::env::var(var) {
            log(&format!("{:23}: {}", var, value), debug_mode);
        }
    }

    // フックイベント名を取得（コマンドライン引数 > stdin）
    let hook_event_name = args.hook_event
        .or_else(|| hook_input.as_ref().and_then(|i| i.hook_event_name.clone()))
        .unwrap_or_else(|| "UserPromptSubmit".to_string());

    log("", debug_mode);
    log(&format!("【処理対象イベント】: {}", hook_event_name), debug_mode);

    // PreToolUse/PostToolUseは素通し（ログのみ）
    if hook_event_name == "PreToolUse" || hook_event_name == "PostToolUse" {
        log("", debug_mode);
        log("【処理】: ツールフック - 素通し", debug_mode);

        let output = json!({});
        let json_str = serde_json::to_string_pretty(&output).unwrap();

        log("", debug_mode);
        log("【出力情報】", debug_mode);
        log(&format!("JSON総サイズ: {}バイト", json_str.len()), debug_mode);

        println!("{}", json_str);

        log("", debug_mode);
        log("=== フック完了 (素通し) ===", debug_mode);
        log("================================================================================", debug_mode);
        return;
    }

    // UserPromptSubmit: CLAUDE.md読み込み処理
    log("", debug_mode);
    log("【処理】: CLAUDE.md読み込み", debug_mode);

    // グローバルCLAUDE.mdのパスを取得
    let home_dir = match std::env::var("HOME") {
        Ok(dir) => dir,
        Err(e) => {
            log(&format!("ERROR: HOME変数が見つかりません: {}", e), debug_mode);
            eprintln!("HOME environment variable not set");
            log("", debug_mode);
            log("=== フック終了 (エラー) ===", debug_mode);
            log("================================================================================", debug_mode);
            return;
        }
    };

    let global_claude_md: PathBuf = [&home_dir, ".claude", "CLAUDE.md"]
        .iter()
        .collect();

    // プロジェクトCLAUDE.mdのパスを取得
    let project_claude_md: Option<PathBuf> = hook_input.as_ref()
        .and_then(|input| input.cwd.as_ref())
        .and_then(|cwd| {
            let path1: PathBuf = [cwd, "CLAUDE.md"].iter().collect();
            let path2: PathBuf = [cwd, ".claude", "CLAUDE.md"].iter().collect();
            if path1.exists() {
                Some(path1)
            } else if path2.exists() {
                Some(path2)
            } else {
                None // 存在しない場合はNone
            }
        });

    log("", debug_mode);
    log("【ファイル読み込み】", debug_mode);
    log(&format!("グローバル: {}", global_claude_md.display()), debug_mode);
    if let Some(ref proj_path) = project_claude_md {
        log(&format!("プロジェクト: {}", proj_path.display()), debug_mode);
    }

    // グローバルCLAUDE.mdを読み込み
    let global_content = match fs::read_to_string(&global_claude_md) {
        Ok(content) => {
            log(&format!("グローバル: 成功 ({}バイト, {}行)", content.len(), content.lines().count()), debug_mode);
            Some(content)
        }
        Err(e) => {
            log(&format!("グローバル: 失敗 - {}", e), debug_mode);
            None
        }
    };

    // プロジェクトCLAUDE.mdを読み込み
    let project_content = if let Some(ref proj_path) = project_claude_md {
        match fs::read_to_string(proj_path) {
            Ok(content) => {
                log(&format!("プロジェクト: 成功 ({}バイト, {}行)", content.len(), content.lines().count()), debug_mode);
                Some(content)
            }
            Err(e) => {
                log(&format!("プロジェクト: 失敗 - {}", e), debug_mode);
                None
            }
        }
    } else {
        log("プロジェクト: cwd情報なし", debug_mode);
        None
    };

    match (global_content, project_content) {
        (Some(global), Some(project)) => {
            log("", debug_mode);
            log("グローバル内容:", debug_mode);
            for line in global.lines() {
                log(&format!("  {}", line), debug_mode);
            }
            log("", debug_mode);
            log("プロジェクト内容:", debug_mode);
            for line in project.lines() {
                log(&format!("  {}", line), debug_mode);
            }

            // 成功: 両方のCLAUDE.mdの内容を含むJSONを出力
            let additional_context = format!(
                "【必須ルール】以下の内容を最優先で遵守すること:\n\n## グローバル設定 (~/.claude/CLAUDE.md)\n---\n{}\n---\n\n## プロジェクト設定 (CLAUDE.md)\n---\n{}\n---\n\n✅ 上記ルールを読み込み、全ての動作で遵守します",
                global.trim_end(), project.trim_end()
            );

            let output = json!({
                "hookSpecificOutput": {
                    "hookEventName": "UserPromptSubmit",
                    "additionalContext": additional_context
                }
            });

            let json_str = serde_json::to_string_pretty(&output).unwrap();

            log("", debug_mode);
            log("【出力情報】", debug_mode);
            log(&format!("hookEventName       : UserPromptSubmit"), debug_mode);
            log(&format!("additionalContext   : {}バイト (グローバル + プロジェクト)", additional_context.len()), debug_mode);
            log(&format!("JSON総サイズ        : {}バイト", json_str.len()), debug_mode);

            println!("{}", json_str);

            log("", debug_mode);
            log("=== フック完了 (成功) ===", debug_mode);
            log("================================================================================", debug_mode);
        },
        (Some(global), None) => {
            log("", debug_mode);
            log("グローバル内容:", debug_mode);
            for line in global.lines() {
                log(&format!("  {}", line), debug_mode);
            }

            // 成功: グローバルCLAUDE.mdのみ
            let additional_context = format!(
                "【必須ルール】以下の内容を最優先で遵守すること:\n---\n{}\n---\n✅ 上記ルールを読み込み、全ての動作で遵守します",
                global.trim_end()
            );

            let output = json!({
                "hookSpecificOutput": {
                    "hookEventName": "UserPromptSubmit",
                    "additionalContext": additional_context
                },
                "systemMessage": "✅ CLAUDE.md読込完了"
            });

            let json_str = serde_json::to_string_pretty(&output).unwrap();

            log("", debug_mode);
            log("【出力情報】", debug_mode);
            log(&format!("hookEventName       : UserPromptSubmit"), debug_mode);
            log(&format!("additionalContext   : {}バイト (グローバルのみ)", additional_context.len()), debug_mode);
            log(&format!("JSON総サイズ        : {}バイト", json_str.len()), debug_mode);

            println!("{}", json_str);

            log("", debug_mode);
            log("=== フック完了 (成功) ===", debug_mode);
            log("================================================================================", debug_mode);
        },
        (None, Some(project)) => {
            log("", debug_mode);
            log("プロジェクト内容:", debug_mode);
            for line in project.lines() {
                log(&format!("  {}", line), debug_mode);
            }

            // 成功: プロジェクトCLAUDE.mdのみ
            let additional_context = format!(
                "【必須ルール】以下の内容を最優先で遵守すること:\n---\n{}\n---\n✅ 上記ルールを読み込み、全ての動作で遵守します",
                project.trim_end()
            );

            let output = json!({
                "hookSpecificOutput": {
                    "hookEventName": "UserPromptSubmit",
                    "additionalContext": additional_context
                },
                "systemMessage": "✅ CLAUDE.md読込完了"
            });

            let json_str = serde_json::to_string_pretty(&output).unwrap();

            log("", debug_mode);
            log("【出力情報】", debug_mode);
            log(&format!("hookEventName       : UserPromptSubmit"), debug_mode);
            log(&format!("additionalContext   : {}バイト (プロジェクトのみ)", additional_context.len()), debug_mode);
            log(&format!("JSON総サイズ        : {}バイト", json_str.len()), debug_mode);

            println!("{}", json_str);

            log("", debug_mode);
            log("=== フック完了 (成功) ===", debug_mode);
            log("================================================================================", debug_mode);
        },
        (None, None) => {
            // 両方ない場合でもブロックしない
            let output = json!({
                "hookSpecificOutput": {
                    "hookEventName": "UserPromptSubmit",
                    "additionalContext": ""
                },
                "systemMessage": "ℹ️ CLAUDE.mdをスキップ"
            });

            let json_str = serde_json::to_string_pretty(&output).unwrap();

            log("", debug_mode);
            log("【出力情報】", debug_mode);
            log(&format!("hookEventName       : UserPromptSubmit"), debug_mode);
            log(&format!("additionalContext   : (空文字列)"), debug_mode);
            log(&format!("JSON総サイズ        : {}バイト", json_str.len()), debug_mode);

            println!("{}", json_str);

            log("", debug_mode);
            log("=== フック完了 (成功・ファイルなし) ===", debug_mode);
            log("================================================================================", debug_mode);
        }
    }
}
