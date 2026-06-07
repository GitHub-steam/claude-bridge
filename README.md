# ClaudeBridge

> Browse, search, resume and migrate your **Claude Code** conversations across **multiple local accounts** — without switching accounts or losing history.
>
> 在一个界面里浏览、搜索、续聊、并跨账号迁移本机的 **Claude Code** 聊天记录 —— 不用切号、不丢历史。

<p align="center"><em>A small, local, non-destructive desktop tool (Tauri + Rust).</em></p>

---

## 为什么有这个工具 / Why

一台电脑上登录了多个 Claude 账号时，桌面端「Code」的会话列表是**按账号隔离**的：切到 A 号就看不到 B 号聊过的项目，更没有办法把某段对话拿到另一个号继续。

现有的第三方工具几乎都是「只读查看 / 导出 Markdown」——**没有一个能跨账号迁移并继续对话**。ClaudeBridge 填补的就是这个空白。

## 功能 / Features

- 🗂 **全账号浏览**：一个界面看到本机所有账号 + 纯 CLI 的对话，无需切号
- 🔎 **搜索 / 账号筛选**（全文内容搜索开发中）
- 💬 **左右 / 平铺两种对话视图**
- 🖥 **命令行续聊**：一键用本机 `claude --resume` 接着聊（无视账号、不重启）
- 🔀 **迁移到指定账号**：把对话复制到另一个号（写一个几百字节的指针，**不动正文**），重启桌面端后即可在该号续聊；支持一键撤销
- 📤 **导出 Markdown**

## 工作原理 / How it works

Claude 的会话其实分两层存在本机：

| 层 | 位置 | 说明 |
|---|---|---|
| **对话正文** | `~/.claude/projects/<编码cwd>/<id>.jsonl` | 全账号**共享**，跟操作系统用户走 |
| **每账号会话索引** | `%APPDATA%/Claude/claude-code-sessions/<账号>/<org>/local_*.json` | 桌面端侧边栏的来源，**按账号隔离** |

桌面端「换号看不到」只是因为第二层的小指针被分到了不同账号文件夹；正文本来就是共享的。**迁移 = 在目标账号目录新增一个指向共享正文的指针**，因此非破坏、可撤销。

## ⚠️ 范围与免责 / Scope & disclaimer

- ✅ 仅用于**你本人的多个账号**、**本机本地文件**操作。不涉及任何账号共享、不绕过登录鉴权、不调用任何云端 API、**绝不读取登录凭证**。
- ✅ **非破坏性**：只新增/读取指针，从不修改或删除对话正文；写操作前可撤销。
- ⚠️ 依赖 Claude 桌面端**未公开的本地存储格式**，桌面端升级后可能需要适配更新。
- 🧪 已验证版本：Claude 桌面端 `1.11187.x` / `claude-code 2.1.165`（Windows）。其它版本/平台可能需要调整。

## 安装与构建 / Build

前置：[Rust](https://rustup.rs) + [Node.js](https://nodejs.org)（LTS）。

```bash
cd claude-bridge
npm install
npm run tauri dev      # 开发模式运行
npm run tauri build    # 打包可分发安装包
```

跨平台数据位置：Windows `%APPDATA%/Claude`、macOS `~/Library/Application Support/Claude`、Linux `~/.config/Claude`；对话正文统一在 `~/.claude/projects`。

## 路线图 / Roadmap

- [ ] 全文搜索对话内容
- [ ] 日期筛选 + 多选批量导出/迁移
- [ ] 左侧分组/排序视图
- [ ] 转录 Markdown 渲染 + 代码高亮
- [ ] 账号别名 + 设置面板
- [ ] 标注/过滤已归档会话
- [ ] 深色模式
- [ ] （实验）`claude://` 深链原生打开 / 检测更新

## 技术栈 / Stack

Tauri v2 · Rust（扫描 / 解析 / 写盘）· React + TypeScript（前端）。

## License

MIT — see [LICENSE](./LICENSE). 设计与实现细节见 [PRD.md](./PRD.md)。
