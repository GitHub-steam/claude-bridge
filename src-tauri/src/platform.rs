//! 跨平台路径解析：定位 Claude 桌面端数据目录与 CLI 正文目录。

use std::path::PathBuf;

/// 用户主目录
pub fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

/// 桌面端数据根目录
/// - Windows: %APPDATA%\Claude
/// - macOS:   ~/Library/Application Support/Claude
/// - Linux:   $XDG_CONFIG_HOME/Claude 或 ~/.config/Claude
pub fn claude_data_root() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // 1) 经典 Win32 安装：%APPDATA%\Claude
        let classic = std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Claude"));
        if let Some(ref p) = classic {
            if p.is_dir() {
                return classic;
            }
        }
        // 2) 打包版（MSIX/应用商店）：写入 %APPDATA% 会被重定向到
        //    %LOCALAPPDATA%\Packages\<Claude...>\LocalCache\Roaming\Claude
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let packages = PathBuf::from(&local).join("Packages");
            if let Ok(entries) = std::fs::read_dir(&packages) {
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    if name.contains("claude") {
                        let p = e
                            .path()
                            .join("LocalCache")
                            .join("Roaming")
                            .join("Claude");
                        if p.is_dir() {
                            return Some(p);
                        }
                    }
                }
            }
        }
        // 3) 都不存在 → 回退经典路径（用于诊断显示）
        classic
    }
    #[cfg(target_os = "macos")]
    {
        home_dir().map(|h| h.join("Library").join("Application Support").join("Claude"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(x) = std::env::var_os("XDG_CONFIG_HOME") {
            Some(PathBuf::from(x).join("Claude"))
        } else {
            home_dir().map(|h| h.join(".config").join("Claude"))
        }
    }
}

/// 每账号会话索引目录：<data_root>/claude-code-sessions
pub fn sessions_root() -> Option<PathBuf> {
    claude_data_root().map(|r| r.join("claude-code-sessions"))
}

/// CLI 对话正文目录：~/.claude/projects（全平台一致）
pub fn projects_root() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".claude").join("projects"))
}

/// 把工作目录 cwd 编码为 projects 下的文件夹名：
/// 每个非 [a-zA-Z0-9] 字符替换为 '-'。
/// 例：`D:\projects\my-app` -> `D--projects-my-app`
pub fn encode_cwd(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}
