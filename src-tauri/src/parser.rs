//! 把一个 JSONL 正文文件解析为可读的消息列表（详情视图用）。

use serde::Serialize;
use serde_json::Value;
use std::fs;

#[derive(Serialize)]
pub struct Message {
    pub role: String,
    pub text: String,
    pub tools: Vec<String>, // 仅 assistant 的工具调用名（不含 emoji）
    pub timestamp: Option<String>,
}

pub fn parse_transcript(path: &str) -> Result<Vec<Message>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let t = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
        if t != "user" && t != "assistant" {
            continue;
        }
        let msg = match v.get("message") {
            Some(m) => m,
            None => continue,
        };
        let role = msg
            .get("role")
            .and_then(|x| x.as_str())
            .unwrap_or(t)
            .to_string();

        let mut text = String::new();
        let mut tools: Vec<String> = Vec::new();

        match msg.get("content") {
            Some(Value::String(s)) => text.push_str(s),
            Some(Value::Array(arr)) => {
                for b in arr {
                    match b.get("type").and_then(|x| x.as_str()).unwrap_or("") {
                        "text" => {
                            if let Some(s) = b.get("text").and_then(|x| x.as_str()) {
                                if !text.is_empty() {
                                    text.push('\n');
                                }
                                text.push_str(s);
                            }
                        }
                        "tool_use" => {
                            if let Some(name) = b.get("name").and_then(|x| x.as_str()) {
                                tools.push(name.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        let text = text.trim().to_string();

        // 用户回合：只保留真正有文字的输入，丢弃「纯 tool_result 回显」的噪音
        if role == "user" {
            if text.is_empty() {
                continue;
            }
            tools.clear();
        } else {
            // assistant：有文字或有工具调用才保留
            if text.is_empty() && tools.is_empty() {
                continue;
            }
        }

        out.push(Message {
            role,
            text,
            tools,
            timestamp: v.get("timestamp").and_then(|x| x.as_str()).map(|s| s.to_string()),
        });
    }

    Ok(out)
}
