//! ClaudeBridge — 后端命令入口

mod actions;
mod parser;
mod platform;
mod scanner;

/// 列出本机所有对话（跨账号）。async → 在独立线程执行，避免阻塞 UI 主线程
#[tauri::command]
async fn list_conversations() -> Vec<scanner::Conversation> {
    scanner::scan_conversations()
}

/// 列出所有账号 + 会话数
#[tauri::command]
async fn list_accounts() -> Vec<scanner::AccountInfo> {
    scanner::scan_accounts()
}

/// 读取某条对话的完整内容（解析为消息列表）
#[tauri::command]
fn get_transcript(path: String) -> Result<Vec<parser::Message>, String> {
    if !actions::within_projects(&path) {
        return Err("路径越界：仅允许读取 projects 目录内的对话".into());
    }
    parser::parse_transcript(&path)
}

/// 全文搜索对话正文。async → 不阻塞 UI 主线程
#[tauri::command]
async fn search_content(query: String, account_id: Option<String>) -> Vec<scanner::ContentHit> {
    scanner::search_content(query, account_id)
}

/// 在新终端中 `claude --resume` 续聊（可指定 claude 二进制路径）
#[tauri::command]
fn resume_session(
    cli_session_id: String,
    cwd: Option<String>,
    claude_bin: Option<String>,
) -> Result<(), String> {
    actions::resume_in_terminal(cli_session_id, cwd, claude_bin)
}

/// 诊断：解析后的目录/计数/claude 路径
#[tauri::command]
fn diagnostics() -> scanner::Diagnostics {
    scanner::diagnostics()
}

/// 迁移一条对话到指定账号（写指针）
#[tauri::command]
fn migrate_session(
    cli_session_id: String,
    cwd: String,
    title: Option<String>,
    model: Option<String>,
    target_account_id: String,
    target_org_id: String,
) -> Result<actions::MigrateResult, String> {
    actions::migrate_session(cli_session_id, cwd, title, model, target_account_id, target_org_id)
}

/// 撤销上一次迁移（删除新增的指针）
#[tauri::command]
fn undo_migrate(file_path: String) -> Result<(), String> {
    actions::undo_migrate(file_path)
}

/// 导出一条对话为 Markdown
#[tauri::command]
fn export_markdown(transcript_path: String, title: Option<String>) -> Result<String, String> {
    if !actions::within_projects(&transcript_path) {
        return Err("路径越界：仅允许导出 projects 目录内的对话".into());
    }
    actions::export_markdown(transcript_path, title)
}

/// 在文件管理器中打开目录
#[tauri::command]
fn reveal_path(path: String) -> Result<(), String> {
    actions::reveal_path(path)
}

/// 在浏览器打开链接
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    actions::open_url(url)
}

/// 手动指定账号会话目录（留空 = 回到自动探测）
#[tauri::command]
fn set_sessions_override(path: Option<String>) {
    platform::set_sessions_override(path);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_conversations,
            list_accounts,
            get_transcript,
            search_content,
            resume_session,
            migrate_session,
            undo_migrate,
            export_markdown,
            diagnostics,
            reveal_path,
            open_url,
            set_sessions_override
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
