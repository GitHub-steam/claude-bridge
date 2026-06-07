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
    pub archived: bool,
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
                    let archived = !accounts.is_empty() && accounts.iter().all(|a| a.is_archived);

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
                        archived,
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

/// 列出所有账号/组织（直接枚举目录，连「零会话」的全新账号也包含进来，
/// 这样迁移时可以选到一个还没有任何会话的新号）+ 各自会话数
pub fn scan_accounts() -> Vec<AccountInfo> {
    let mut out: Vec<AccountInfo> = Vec::new();
    let root = match platform::sessions_root() {
        Some(r) => r,
        None => return out,
    };
    if let Ok(accts) = fs::read_dir(&root) {
        for acct in accts.flatten() {
            if !acct.path().is_dir() {
                continue;
            }
            let account_id = acct.file_name().to_string_lossy().to_string();
            if let Ok(orgs) = fs::read_dir(acct.path()) {
                for org in orgs.flatten() {
                    if !org.path().is_dir() {
                        continue;
                    }
                    let org_id = org.file_name().to_string_lossy().to_string();
                    let mut count = 0usize;
                    if let Ok(files) = fs::read_dir(org.path()) {
                        for f in files.flatten() {
                            let n = f.file_name().to_string_lossy().to_string();
                            if n.starts_with("local_") && n.ends_with(".json") {
                                count += 1;
                            }
                        }
                    }
                    out.push(AccountInfo {
                        account_id: account_id.clone(),
                        org_id,
                        session_count: count,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| b.session_count.cmp(&a.session_count));
    out
}

/// 全文搜索命中
#[derive(Serialize, Clone)]
pub struct ContentHit {
    pub cli_session_id: String,
    pub title: Option<String>,
    pub match_count: usize,
    pub snippet: String,
}

fn floor_boundary(s: &str, mut i: usize) -> usize {
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}
fn ceil_boundary(s: &str, mut i: usize) -> usize {
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// 在匹配处附近取一段上下文片段（UTF-8 边界安全）
fn make_snippet(text: &str, match_byte: usize, mlen: usize) -> String {
    const CTX: usize = 44;
    let mb = match_byte.min(text.len());
    let s = floor_boundary(text, mb.saturating_sub(CTX));
    let e = ceil_boundary(text, (mb + mlen + CTX).min(text.len()));
    let mut out = String::new();
    if s > 0 {
        out.push('…');
    }
    out.push_str(text[s..e].trim());
    if e < text.len() {
        out.push('…');
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 流式扫描单个正文文件，统计匹配数 + 首个片段
fn search_file(path: &str, q_lower: &str) -> (usize, String) {
    use std::io::BufRead;
    let mut count = 0usize;
    let mut snippet = String::new();
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return (0, String::new()),
    };
    let reader = std::io::BufReader::new(file);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let t = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
        if t != "user" && t != "assistant" {
            continue;
        }
        let mut hay = extract_text(&v).unwrap_or_default();
        if let Some(arr) = v
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|x| x.as_array())
        {
            for b in arr {
                if b.get("type").and_then(|x| x.as_str()) == Some("tool_use") {
                    if let Some(n) = b.get("name").and_then(|x| x.as_str()) {
                        hay.push(' ');
                        hay.push_str(n);
                    }
                }
            }
        }
        if hay.is_empty() {
            continue;
        }
        let hl = hay.to_lowercase();
        let mut start = 0usize;
        while let Some(pos) = hl[start..].find(q_lower) {
            count += 1;
            let abs = start + pos;
            if snippet.is_empty() {
                snippet = make_snippet(&hay, abs, q_lower.len());
            }
            start = abs + q_lower.len();
            if start >= hl.len() || count > 9999 {
                break;
            }
        }
    }
    (count, snippet)
}

/// 全文搜索所有对话的正文（user/assistant 文本 + 工具名）
pub fn search_content(query: String, account_id: Option<String>) -> Vec<ContentHit> {
    let q = query.trim().to_lowercase();
    if q.chars().count() < 2 {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for c in scan_conversations() {
        if let Some(ref acc) = account_id {
            if !c.accounts.iter().any(|a| &a.account_id == acc) {
                continue;
            }
        }
        let (count, snippet) = search_file(&c.transcript_path, &q);
        if count > 0 {
            hits.push(ContentHit {
                cli_session_id: c.cli_session_id,
                title: c.title,
                match_count: count,
                snippet,
            });
        }
    }
    hits.sort_by(|a, b| b.match_count.cmp(&a.match_count));
    hits
}

#[derive(Serialize)]
pub struct PathInfo {
    pub path: String,
    pub exists: bool,
    pub entry_count: usize,
}

#[derive(Serialize)]
pub struct Diagnostics {
    pub projects_root: PathInfo,
    pub sessions_root: PathInfo,
    pub transcript_count: usize,
    pub pointer_count: usize,
    pub account_count: usize,
    pub claude_bin: String,
}

/// 诊断：解析后的目录、是否存在、各类计数、解析出的 claude 路径
pub fn diagnostics() -> Diagnostics {
    let proot = platform::projects_root();
    let sroot = platform::sessions_root();

    let mut proj_dirs = 0usize;
    let mut transcript_count = 0usize;
    let (p_path, p_exists) = match &proot {
        Some(p) => (p.to_string_lossy().to_string(), p.is_dir()),
        None => (String::new(), false),
    };
    if let Some(ref p) = proot {
        if let Ok(dirs) = fs::read_dir(p) {
            for d in dirs.flatten() {
                if d.path().is_dir() {
                    proj_dirs += 1;
                    if let Ok(files) = fs::read_dir(d.path()) {
                        for f in files.flatten() {
                            if f.path().is_file()
                                && f.file_name().to_string_lossy().ends_with(".jsonl")
                            {
                                transcript_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    let mut acct_dirs = 0usize;
    let (s_path, s_exists) = match &sroot {
        Some(s) => (s.to_string_lossy().to_string(), s.is_dir()),
        None => (String::new(), false),
    };
    if let Some(ref s) = sroot {
        if let Ok(dirs) = fs::read_dir(s) {
            for d in dirs.flatten() {
                if d.path().is_dir() {
                    acct_dirs += 1;
                }
            }
        }
    }

    Diagnostics {
        projects_root: PathInfo {
            path: p_path,
            exists: p_exists,
            entry_count: proj_dirs,
        },
        sessions_root: PathInfo {
            path: s_path,
            exists: s_exists,
            entry_count: acct_dirs,
        },
        transcript_count,
        pointer_count: read_pointers().len(),
        account_count: acct_dirs,
        claude_bin: crate::actions::find_claude(),
    }
}
