//! 扫描本机所有账号的会话指针 + CLI 正文，建立统一索引。

use crate::platform;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 某条对话被某个账号收录的引用
#[derive(Serialize, Clone)]
pub struct AccountRef {
    pub account_id: String,
    pub org_id: String,
    pub session_id: String,
    pub title: Option<String>,
    pub is_archived: bool,
}

/// 账号概览
#[derive(Serialize, Clone)]
pub struct AccountInfo {
    pub account_id: String,
    pub org_id: String,
    pub session_count: usize,
}

/// 一条对话（以 cliSessionId / 正文文件为主键）
#[derive(Serialize, Clone)]
pub struct Conversation {
    pub cli_session_id: String,
    pub transcript_path: String,
    pub project_dir: String,
    pub cwd: Option<String>,
    pub title: Option<String>,
    pub model: Option<String>,
    pub created_at: Option<i64>,
    pub last_activity_at: Option<i64>,
    pub message_count: usize,
    pub size_bytes: u64,
    pub accounts: Vec<AccountRef>,
    pub has_pointer: bool,
}

struct PointerMeta {
    cli_session_id: String,
    cwd: Option<String>,
    title: Option<String>,
    model: Option<String>,
    created_at: Option<i64>,
    last_activity_at: Option<i64>,
    account: AccountRef,
}

fn num_to_i64(v: &Value) -> Option<i64> {
    if let Some(i) = v.as_i64() {
        Some(i)
    } else if let Some(u) = v.as_u64() {
        Some(u as i64)
    } else {
        v.as_f64().map(|f| f as i64)
    }
}

/// 读取所有账号目录下的 local_*.json 指针
fn read_pointers() -> Vec<PointerMeta> {
    let mut out = Vec::new();
    let root = match platform::sessions_root() {
        Some(r) => r,
        None => return out,
    };
    let accts = match fs::read_dir(&root) {
        Ok(a) => a,
        Err(_) => return out,
    };
    for acct in accts.flatten() {
        if !acct.path().is_dir() {
            continue;
        }
        let account_id = acct.file_name().to_string_lossy().to_string();
        let orgs = match fs::read_dir(acct.path()) {
            Ok(o) => o,
            Err(_) => continue,
        };
        for org in orgs.flatten() {
            if !org.path().is_dir() {
                continue;
            }
            let org_id = org.file_name().to_string_lossy().to_string();
            let files = match fs::read_dir(org.path()) {
                Ok(f) => f,
                Err(_) => continue,
            };
            for f in files.flatten() {
                let name = f.file_name().to_string_lossy().to_string();
                if !(name.starts_with("local_") && name.ends_with(".json")) {
                    continue;
                }
                let data = match fs::read_to_string(f.path()) {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                let v: Value = match serde_json::from_str(&data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let cli = match v.get("cliSessionId").and_then(|x| x.as_str()) {
                    Some(c) => c.to_string(),
                    None => continue,
                };
                let session_id = v
                    .get("sessionId")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let title = v
                    .get("title")
                    .and_then(|x| x.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.to_string());
                let is_archived = v.get("isArchived").and_then(|x| x.as_bool()).unwrap_or(false);
                out.push(PointerMeta {
                    cli_session_id: cli,
                    cwd: v.get("cwd").and_then(|x| x.as_str()).map(|s| s.to_string()),
                    title: title.clone(),
                    model: v.get("model").and_then(|x| x.as_str()).map(|s| s.to_string()),
                    created_at: v.get("createdAt").and_then(num_to_i64),
                    last_activity_at: v.get("lastActivityAt").and_then(num_to_i64),
                    account: AccountRef {
                        account_id: account_id.clone(),
                        org_id: org_id.clone(),
                        session_id,
                        title,
                        is_archived,
                    },
                });
            }
        }
    }
    out
}

/// 从正文中提取一段文本（用于无标题时的预览）
fn extract_text(line: &Value) -> Option<String> {
    let msg = line.get("message")?;
    let content = msg.get("content")?;
    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = content.as_array() {
        let mut buf = String::new();
        for block in arr {
            if block.get("type").and_then(|x| x.as_str()) == Some("text") {
                if let Some(txt) = block.get("text").and_then(|x| x.as_str()) {
                    buf.push_str(txt);
                }
            }
        }
        if !buf.trim().is_empty() {
            return Some(buf);
        }
    }
    None
}

/// quick_scan 的产物
struct ScanInfo {
    count: usize,
    first_user: Option<String>,
    /// 从正文里发现的 cwd（指针缺失时兜底）
    cwd: Option<String>,
    /// 文件 mtime（毫秒），指针缺时间戳时兜底
    last_ts: Option<i64>,
}

/// 流式扫描一个正文文件：消息数 + 首条用户预览 + cwd 兜底 + mtime 兜底
fn quick_scan(path: &Path) -> ScanInfo {
    use std::io::BufRead;

    let mut info = ScanInfo {
        count: 0,
        first_user: None,
        cwd: None,
        last_ts: None,
    };

    if let Ok(file) = fs::File::open(path) {
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue, // 跳过坏行（如非 UTF-8），继续扫描后续
            };
            if line.trim().is_empty() {
                continue;
            }
            let v: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            // 任意带 cwd 的行都可作兜底来源
            if info.cwd.is_none() {
                if let Some(c) = v.get("cwd").and_then(|x| x.as_str()) {
                    if !c.trim().is_empty() {
                        info.cwd = Some(c.to_string());
                    }
                }
            }
            let t = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
            if t == "user" || t == "assistant" {
                info.count += 1;
                if t == "user" && info.first_user.is_none() {
                    if let Some(txt) = extract_text(&v) {
                        let s: String = txt.split_whitespace().collect::<Vec<_>>().join(" ");
                        let s: String = s.chars().take(80).collect();
                        if !s.trim().is_empty() {
                            info.first_user = Some(s);
                        }
                    }
                }
            }
        }
    }

    if let Ok(meta) = fs::metadata(path) {
        if let Ok(modified) = meta.modified() {
            if let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) {
                info.last_ts = Some(dur.as_millis() as i64);
            }
        }
    }

    info
}

/// 主入口：扫描所有对话
pub fn scan_conversations() -> Vec<Conversation> {
    let pointers = read_pointers();
    let mut by_cli: HashMap<String, Vec<PointerMeta>> = HashMap::new();
    for p in pointers {
        by_cli.entry(p.cli_session_id.clone()).or_default().push(p);
    }

    let mut convos: Vec<Conversation> = Vec::new();
    // cli_session_id -> convos 索引，用于跨项目目录去重
    let mut idx_map: HashMap<String, usize> = HashMap::new();

    if let Some(proot) = platform::projects_root() {
        if let Ok(dirs) = fs::read_dir(&proot) {
            for d in dirs.flatten() {
                if !d.path().is_dir() {
                    continue;
                }
                let project_dir = d.file_name().to_string_lossy().to_string();
                let files = match fs::read_dir(d.path()) {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                for f in files.flatten() {
                    // 只取顶层 .jsonl（subagents 子目录不会被 read_dir 当成文件遍历进来）
                    if !f.path().is_file() {
                        continue;
                    }
                    let fname = f.file_name().to_string_lossy().to_string();
                    if !fname.ends_with(".jsonl") {
                        continue;
                    }
                    let cli = fname.trim_end_matches(".jsonl").to_string();
                    let path = f.path();
                    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    let ScanInfo {
                        count,
                        first_user,
                        cwd: scanned_cwd,
                        last_ts,
                    } = quick_scan(&path);

                    let ptrs = by_cli.get(&cli);
                    let accounts: Vec<AccountRef> = ptrs
                        .map(|v| v.iter().map(|p| p.account.clone()).collect())
                        .unwrap_or_default();
                    let pmeta = ptrs.and_then(|v| v.first());

                    let conv = Conversation {
                        cli_session_id: cli.clone(),
                        transcript_path: path.to_string_lossy().to_string(),
                        project_dir: project_dir.clone(),
                        cwd: pmeta.and_then(|p| p.cwd.clone()).or(scanned_cwd),
                        title: pmeta.and_then(|p| p.title.clone()).or(first_user),
                        model: pmeta.and_then(|p| p.model.clone()),
                        created_at: pmeta.and_then(|p| p.created_at).or(last_ts),
                        last_activity_at: pmeta.and_then(|p| p.last_activity_at).or(last_ts),
                        message_count: count,
                        size_bytes: size,
                        accounts,
                        has_pointer: ptrs.is_some(),
                    };
                    // 去重：同一 cli 可能落在多个项目目录，保留正文更大的那条
                    match idx_map.get(&cli) {
                        Some(&i) => {
                            if convos[i].size_bytes < conv.size_bytes {
                                convos[i] = conv;
                            }
                        }
                        None => {
                            idx_map.insert(cli.clone(), convos.len());
                            convos.push(conv);
                        }
                    }
                }
            }
        }
    }

    convos.sort_by(|a, b| {
        b.last_activity_at
            .unwrap_or(0)
            .cmp(&a.last_activity_at.unwrap_or(0))
            .then(b.size_bytes.cmp(&a.size_bytes))
    });
    convos
}

/// 列出所有账号 + 各自会话数
pub fn scan_accounts() -> Vec<AccountInfo> {
    let mut map: HashMap<(String, String), usize> = HashMap::new();
    for p in read_pointers() {
        *map.entry((p.account.account_id.clone(), p.account.org_id.clone()))
            .or_insert(0) += 1;
    }
    let mut out: Vec<AccountInfo> = map
        .into_iter()
        .map(|((a, o), c)| AccountInfo {
            account_id: a,
            org_id: o,
            session_count: c,
        })
        .collect();
    out.sort_by(|a, b| b.session_count.cmp(&a.session_count));
    out
}
