//! 写盘动作：CLI 一键续聊 + 跨账号迁移（写指针）+ 撤销。

use crate::platform;
use serde::Serialize;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 把 "2.1.165" 解析成 [2,1,165] 以便数值（而非字符串）比较版本
fn ver_key(s: &str) -> Vec<u64> {
    s.split('.')
        .map(|seg| {
            let digits: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<u64>().unwrap_or(0)
        })
        .collect()
}

/// 会话 ID 只允许 UUID 风格字符，杜绝命令注入
fn valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// 路径段必须是「单层普通名字」，杜绝 `..`、分隔符、绝对路径导致的目录穿越
fn safe_seg(s: &str) -> bool {
    !s.is_empty()
        && !s.contains('/')
        && !s.contains('\\')
        && s != "."
        && s != ".."
        && !std::path::Path::new(s).is_absolute()
}

/// 定位可用的 claude CLI：优先桌面端内置版本，其次 PATH 中的 `claude`
pub fn find_claude() -> String {
    if let Some(root) = platform::claude_data_root() {
        let cc = root.join("claude-code");
        if let Ok(vers) = fs::read_dir(&cc) {
            // (版本目录名, exe 路径)，取版本名字符串最大的
            let mut best: Option<(String, PathBuf)> = None;
            for v in vers.flatten() {
                let ver_name = v.file_name().to_string_lossy().to_string();
                let exe = v.path().join(if cfg!(windows) { "claude.exe" } else { "claude" });
                if exe.exists() {
                    let replace = match &best {
                        Some((bn, _)) => ver_key(&ver_name) > ver_key(bn),
                        None => true,
                    };
                    if replace {
                        best = Some((ver_name, exe));
                    }
                }
            }
            if let Some((_, p)) = best {
                return p.to_string_lossy().to_string();
            }
        }
    }
    "claude".to_string()
}

/// 在新终端窗口中以 `claude --resume <id>` 续聊（无视账号、无需重启桌面端）
pub fn resume_in_terminal(
    cli_session_id: String,
    cwd: Option<String>,
    bin_override: Option<String>,
) -> Result<(), String> {
    if !valid_session_id(&cli_session_id) {
        return Err("非法的会话 ID".into());
    }
    let dir = cwd
        .filter(|s| !s.trim().is_empty())
        .or_else(|| platform::home_dir().map(|h| h.to_string_lossy().to_string()))
        .unwrap_or_else(|| ".".to_string());
    let claude = bin_override
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(find_claude);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        // 直接以 argv 形式启动，绝不拼 shell 字符串（无注入面）
        std::process::Command::new(&claude)
            .current_dir(&dir)
            .args(["--resume", &cli_session_id])
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()
            .map_err(|e| format!("启动失败（claude: {}）：{}", claude, e))?;
    }
    #[cfg(target_os = "macos")]
    {
        // id 已校验。dir/claude 先按 shell 单引号上下文转义（' -> '\''），
        // 再按 AppleScript 双引号字符串转义（\ 和 "），保留路径里的反斜杠/特殊字符。
        let sh = |s: &str| s.replace('\'', "'\\''");
        let inner = format!("cd '{}' && '{}' --resume {}", sh(&dir), sh(&claude), cli_session_id);
        let script = format!(
            "tell application \"Terminal\" to do script \"{}\"",
            inner.replace('\\', "\\\\").replace('"', "\\\"")
        );
        std::process::Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // dir/claude 经环境变量传入，命令体不含可注入内容；id 已校验
        let bashcmd = format!(
            "cd \"$CB_DIR\" && \"$CB_CLAUDE\" --resume {}; exec bash",
            cli_session_id
        );
        std::process::Command::new("x-terminal-emulator")
            .args(["-e", "bash", "-c", &bashcmd])
            .env("CB_DIR", &dir)
            .env("CB_CLAUDE", &claude)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Serialize)]
pub struct MigrateResult {
    pub session_id: String,
    pub file_path: String,
}

/// 迁移：在目标账号目录写入一个指向现有正文的指针（非破坏性，只新增文件）
pub fn migrate_session(
    cli_session_id: String,
    cwd: String,
    title: Option<String>,
    model: Option<String>,
    target_account_id: String,
    target_org_id: String,
) -> Result<MigrateResult, String> {
    if !valid_session_id(&cli_session_id) {
        return Err("非法的会话 ID".into());
    }
    if !safe_seg(&target_account_id) || !safe_seg(&target_org_id) {
        return Err("非法的账号/组织标识".into());
    }
    if cwd.trim().is_empty() {
        return Err("该对话缺少工作目录（cwd），无法迁移".into());
    }

    // 安全校验：确认正文确实存在（共享于 ~/.claude/projects）
    let proot = platform::projects_root().ok_or("找不到 projects 目录")?;
    let encoded = platform::encode_cwd(&cwd);
    let primary = proot.join(&encoded).join(format!("{}.jsonl", cli_session_id));
    let mut exists = primary.exists();
    if !exists {
        if let Ok(dirs) = fs::read_dir(&proot) {
            for d in dirs.flatten() {
                if d.path().join(format!("{}.jsonl", cli_session_id)).exists() {
                    exists = true;
                    break;
                }
            }
        }
    }
    if !exists {
        return Err(format!("找不到对话正文：{}.jsonl", cli_session_id));
    }

    let sroot = platform::sessions_root().ok_or("找不到 claude-code-sessions 目录")?;
    let target_dir = sroot.join(&target_account_id).join(&target_org_id);
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
    // 防御纵深：确认目标目录确实落在 claude-code-sessions 内
    {
        let canon_root = sroot.canonicalize().map_err(|e| e.to_string())?;
        let canon_target = target_dir.canonicalize().map_err(|e| e.to_string())?;
        if !canon_target.starts_with(&canon_root) {
            return Err("拒绝：目标目录越界".into());
        }
    }

    // 后端防重复：目标账号若已有指向同一对话的指针，则不再新增
    if let Ok(entries) = fs::read_dir(&target_dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("local_") && name.ends_with(".json") {
                if let Ok(txt) = fs::read_to_string(e.path()) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                        if v.get("cliSessionId").and_then(|x| x.as_str())
                            == Some(cli_session_id.as_str())
                        {
                            return Err("目标账号已存在该对话".into());
                        }
                    }
                }
            }
        }
    }

    let new_id = format!("local_{}", Uuid::new_v4());
    let now = now_ms();
    let title_final = title.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| cwd.clone());
    let model_final = model
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "claude-opus-4-8".to_string());

    let ptr = json!({
        "sessionId": new_id.clone(),
        "cliSessionId": cli_session_id,
        "cwd": cwd.clone(),
        "originCwd": cwd,
        "lastFocusedAt": now,
        "createdAt": now,
        "lastActivityAt": now,
        "model": model_final,
        "effort": "high",
        "isArchived": false,
        "title": title_final,
        "titleSource": "auto",
        "permissionMode": "bypassPermissions",
        "remoteMcpServersConfig": [],
        "chromePermissionMode": "skip_all_permission_checks",
        "completedTurns": 1,
        "alwaysAllowedReasons": [],
        "sessionPermissionUpdates": [],
        "classifierSummaryEnabled": true
    });

    let final_path = target_dir.join(format!("{}.json", new_id));
    let tmp_path = target_dir.join(format!("{}.json.tmp", new_id));
    let data = serde_json::to_string(&ptr).map_err(|e| e.to_string())?;
    fs::write(&tmp_path, data).map_err(|e| e.to_string())?;
    fs::rename(&tmp_path, &final_path).map_err(|e| e.to_string())?;

    Ok(MigrateResult {
        session_id: new_id,
        file_path: final_path.to_string_lossy().to_string(),
    })
}

/// 撤销迁移：删除刚写入的指针（严格校验路径，绝不误删）
pub fn undo_migrate(file_path: String) -> Result<(), String> {
    let sroot = platform::sessions_root().ok_or("找不到会话目录")?;
    let p = PathBuf::from(&file_path);

    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if !(name.starts_with("local_") && name.ends_with(".json")) {
        return Err("拒绝：不是会话指针文件".into());
    }
    if !p.exists() {
        return Ok(()); // 已被删除：幂等返回成功
    }
    // 必须位于 claude-code-sessions 目录内
    let canon_root = sroot.canonicalize().map_err(|e| e.to_string())?;
    let canon_p = p.canonicalize().map_err(|e| e.to_string())?;
    if !canon_p.starts_with(&canon_root) {
        return Err("拒绝：路径不在会话目录内".into());
    }
    fs::remove_file(&canon_p).map_err(|e| e.to_string())?;
    Ok(())
}

/// 将一条对话导出为 Markdown 文件，返回导出路径
pub fn export_markdown(transcript_path: String, title: Option<String>) -> Result<String, String> {
    let msgs = crate::parser::parse_transcript(&transcript_path)?;
    let title = title
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "conversation".to_string());

    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", title));
    md.push_str(&format!(
        "> 从 ClaudeBridge 导出 · 共 {} 条消息\n\n---\n\n",
        msgs.len()
    ));
    for m in &msgs {
        let who = if m.role == "user" { "你" } else { "Claude" };
        md.push_str(&format!("**{}**\n\n", who));
        if !m.text.is_empty() {
            md.push_str(&m.text);
            md.push_str("\n\n");
        }
        if !m.tools.is_empty() {
            md.push_str(&format!("*工具调用：{}*\n\n", m.tools.join(", ")));
        }
        md.push_str("---\n\n");
    }

    let home = platform::home_dir().ok_or("找不到用户目录")?;
    let downloads = home.join("Downloads");
    let dir = if downloads.is_dir() { downloads } else { home };

    let safe: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let safe = safe.trim().chars().take(60).collect::<String>();
    let safe = if safe.is_empty() { "conversation".to_string() } else { safe };

    let mut path = dir.join(format!("{}.md", safe));
    let mut n = 1;
    while path.exists() {
        path = dir.join(format!("{}-{}.md", safe, n));
        n += 1;
        if n > 9999 {
            break;
        }
    }

    fs::write(&path, md).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}
