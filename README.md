# ClaudeBridge

> 在一个界面里浏览、搜索、续聊、并跨账号迁移本机的 **Claude Code** 聊天记录 —— 不用切号、不丢历史。
>
> Browse, search, resume and migrate your **Claude Code** conversations across multiple local accounts.

本地运行的小工具（Tauri + Rust + React），不上传任何数据。

## 为什么 / Why

一台电脑登录多个 Claude 账号时，桌面端的会话列表是按账号分开的：切到一个号就看不到另一个号的项目，也没办法把某段对话拿到别的号继续。ClaudeBridge 把本机各账号的记录汇总到一处，方便查看、搜索、续聊与迁移。

## 功能 / Features

- 全账号浏览（含纯 CLI 会话），无需切号
- 搜索：标题/路径 + 全文内容
- 左右 / 平铺两种对话视图，Markdown 渲染 + 代码高亮
- 命令行一键续聊
- 跨账号迁移（非破坏性、可撤销）
- 导出 Markdown（单条 / 批量）
- 深色模式、设置面板（账号别名、数据诊断等）

## 范围与免责 / Scope

- 仅用于**你本人的多个账号**、**本机本地操作**；不涉及账号共享、不绕过登录、不调用任何云端、**不读取登录凭证**。
- **非破坏性**：只新增/读取索引，从不修改或删除对话正文；迁移可一键撤销。
- 依赖桌面端的本地存储，桌面端升级后可能需要适配更新。

## 构建 / Build

前置：[Rust](https://rustup.rs) + [Node.js](https://nodejs.org)（LTS）。

```bash
cd claude-bridge
npm install
npm run tauri dev      # 开发运行
npm run tauri build    # 打包
```

## License

MIT © 2026 ikan
