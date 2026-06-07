# PRD — ClaudeBridge（Claude 多账号会话迁移工具）

> 一句话：一个本地开源小工具，扫描本机 Claude（桌面端 + CLI）的所有账号聊天记录，可视化浏览，并把任意一条对话「迁移/恢复」到你想要的账号下继续聊。

- 文档版本：v0.1（草案）
- 日期：2026-06-07
- 状态：技术可行性已在真机端到端验证通过 ✅

---

## 1. 背景与痛点

很多人一台电脑上登录了多个 Claude 账号（个人号、工作号、不同订阅额度的号），在桌面端「Code」里轮流办公。问题是：

- **桌面端的会话列表是按账号隔离的**——切到 A 号看不到 B 号聊过的项目对话。
- 想把某个号的某段对话「拿到另一个号继续」，官方没有提供任何入口。
- 现有第三方工具全是「只读查看器 / 导出 Markdown」，**没有一个能把对话回灌到正在用的客户端、更别说跨账号**。

这是一个真实存在、且市面无解的缝隙。

---

## 2. 关键技术发现（已逆向 + 真机验证）

> 这一节是本项目的「地基」，全部经过实测验证，开发时直接照此实现。

### 2.1 数据分两层存储

**第一层：对话正文（全账号共享）**
```
~/.claude/projects/<编码后的cwd>/<cliSessionId>.jsonl
```
- 一行一条消息的 JSONL，是对话的真身。
- **跟着操作系统用户走，不绑 Claude 账号**——所有账号的正文都在同一处，任何本地程序可读。
- `<编码后的cwd>` 编码规则：把工作目录字符串里**每个非 `[a-zA-Z0-9]` 字符替换为 `-`**。
  - 例：`D:\projects\my-app` → `D--projects-my-app`；`N:\` → `N--`；含中文路径每个汉字 → 一个 `-`。

**第二层：桌面端「每账号」的会话索引（按账号隔离）**
```
%APPDATA%\Claude\claude-code-sessions\<accountId>\<orgId>\local_<sessionId>.json
```
- 几百字节的「指针/元数据」文件，**这就是桌面端侧边栏列表的真相来源**（启动时扫描此目录）。
- 桌面端「换号看不到记录」的根因，就是这些指针被分到了不同 `<accountId>` 文件夹；正文其实是共享的。

### 2.2 指针文件 schema（local_*.json）

```json
{
  "sessionId": "local_<uuid>",              // 桌面端会话ID，文件名 = 此值 + .json
  "cliSessionId": "<uuid>",                 // 指向第一层正文文件名（关键关联键）
  "cwd": "D:\\projects\\my-app",               // 工作目录，桌面端据此定位正文文件夹
  "originCwd": "D:\\projects\\my-app",
  "lastFocusedAt": 1780417668913,           // 毫秒时间戳
  "createdAt": 1780393513070,
  "lastActivityAt": 1780393609368,
  "model": "claude-opus-4-8[1m]",
  "effort": "max",
  "isArchived": false,
  "title": "Refactor the auth module",
  "titleSource": "auto",
  "permissionMode": "bypassPermissions",
  "remoteMcpServersConfig": [],
  "chromePermissionMode": "skip_all_permission_checks",
  "completedTurns": 1,
  "alwaysAllowedReasons": [],
  "sessionPermissionUpdates": [],
  "classifierSummaryEnabled": true
}
```

### 2.3 「迁移」的本质

把对话从 B 号弄到 A 号 = **在 A 号的指针目录里新增一个 `local_*.json`，其 `cliSessionId` 指向那条共享正文**。
- 正文文件**完全不动**（共享的），只新增一个几百字节的指针 → 非破坏性、可一键删除还原。
- 复制源指针、只替换 `sessionId`（新 uuid）+ 时间戳即可。

### 2.4 刷新机制（决定交互边界，重要）

- 桌面端只在**账号被（重新）解析时**（= 切号 / 重启 / 重新登录）才调用 `loadSessions()` 重读指针目录。
- **平时不监听该目录**；普通点击、切标签、甚至 `Ctrl+R` 刷新前端**都不会**触发重读（已实测 + 日志确认：会话清单存在主进程内存，前端刷新不动它）。
- 结论：**新增指针后，必须重启（或切号）桌面端才会出现**。「不切不重启、原生侧边栏当场出现」对纯外部工具不可行（IPC 有来源校验、默认无调试端口）。→ 这是产品的**非目标**，见 §4。

### 2.5 CLI 旁路（无视账号）

```
claude --resume <cliSessionId>
```
- CLI 直接读共享的 `~/.claude/projects`，**根本不分账号**——可在任意号下立即续聊任意对话，无需重启、无需迁移。这是「立即续聊」的最佳通道。

### 2.6 跨平台路径

| 平台 | 桌面端数据根 | 正文目录 |
|---|---|---|
| Windows | `%APPDATA%\Claude` | `~/.claude/projects` |
| macOS | `~/Library/Application Support/Claude` | `~/.claude/projects` |
| Linux | `~/.config/Claude` | `~/.claude/projects` |

### 2.7 旁证：官方自带同类能力

桌面端内部有 `registerExternalSession()`（用于从 claude.ai 导入对话：复制正文 → 写指针 → 实时刷新界面）。说明我们走的路子与官方一致、格式受官方使用，稳定性与合规性上站得住。

---

## 3. 目标用户与价值

- **目标用户**：一机多 Claude 账号的开发者 / 重度用户。
- **核心价值**：打破账号墙，让本机所有对话「可见、可搜、可迁移、可续聊」。

---

## 4. 产品目标与非目标

**目标**
1. 扫描并索引本机全部账号的对话（桌面端 + CLI）。
2. 在工具内可视化浏览任意对话全文（不切号、实时）。
3. 一键把对话「迁移」到指定账号（写指针，引导重启生效）。
4. 一键以 CLI 方式立即续聊（无视账号、无需重启）。

**非目标（明确不做）**
- ❌ 「不切号、不重启，让对话当场出现在 Claude 原生侧边栏」——技术上对外部工具不可行（见 §2.4），不投入。
- ❌ 修改 / 删除 / 上传任何对话正文。
- ❌ 触碰登录凭证、调用任何云端 API。

---

## 5. 功能需求

| 编号 | 功能 | 描述 | 优先级 |
|---|---|---|---|
| F1 | 全账号扫描索引 | 遍历 `claude-code-sessions/*/*` 指针 + `~/.claude/projects` 正文，按 `cliSessionId` 关联，识别账号/org | P0 |
| F2 | 可视化浏览 | 列表（标题/账号/项目cwd/时间/消息数/归属号），点开渲染 JSONL 为可读对话 | P0 |
| F3 | 搜索过滤 | 按标题、内容、账号、项目、时间筛选 | P1 |
| F4 | 迁移/恢复到指定账号 | 选目标 `<accountId>/<orgId>`，写指针（原子写、自动备份）；提示「重启桌面端生效」 | P0 |
| F5 | CLI 立即续聊 | 一键起终端执行 `claude --resume <id>`（可附带 cwd） | P1 |
| F6 | 导出 Markdown | 将选中对话导出为 md（对标现有工具的基础能力） | P2 |
| F7 | 备份与撤销 | 任何写操作前备份；提供「撤销上次迁移 / 删除我新增的指针」 | P0 |
| F8 | 账号识别与展示 | 列出本机所有 `accountId/orgId`，标注当前活跃号 | P1 |

---

## 6. 典型用户流程

1. **找回并查看**：打开工具 → 看到全部账号对话 → 搜「Live2D」→ 点开读全文（当前停在 A 号，零操作）。
2. **迁移到本号**：选中 B 号某对话 → 点「迁移到 A 号」→ 工具写指针 + 提示「请重启桌面端」→ 重启后该对话出现在 A 号侧边栏，点开续聊。
3. **懒人续聊**：选中任意对话 → 点「用命令行续聊」→ 弹出终端直接接上，无需重启/迁移。

---

## 7. 技术架构

- **形态**：单文件本地 exe（跨平台）。
- **技术栈（已定）**：**Tauri（Rust 内核 + Web 前端）**——产物小、跨平台、纯本地文件操作契合。
  - Rust 侧（后端命令）：`scanner` / `parser` / `migrator` / `cli-launcher` / `platform` 等文件与系统操作，经 Tauri command 暴露给前端。
  - 前端：建议 React/Svelte + Vite；负责列表、详情渲染、搜索、迁移交互。
- **模块划分**：
  - `scanner`：枚举账号、org、指针、正文；建立关联索引。
  - `parser`：JSONL → 结构化消息（user/assistant/tool 等）。
  - `viewer`（前端）：列表 + 详情渲染 + 搜索。
  - `migrator`：构造并原子写入指针；备份；撤销。
  - `cli-launcher`：拼接并启动 `claude --resume`。
  - `platform`：各 OS 路径解析。
- **数据模型（内存索引）**：
  ```
  Conversation {
    cliSessionId, transcriptPath, cwd, title, model,
    createdAt, lastActivityAt, messageCount,
    presentInAccounts: [ {accountId, orgId, sessionId} ]   // 该对话已被哪些号收录
  }
  ```

---

## 8. 安全与风险

| 风险 | 对策 |
|---|---|
| 写坏指针 | 严格按 schema；先写 `.tmp` 再原子改名；坏文件桌面端会自动跳过（已验证） |
| 误删用户数据 | 只新增指针、永不动正文；所有写操作前备份；提供一键撤销 |
| 格式随版本漂移 | 集中封装路径/schema；版本探测；写入前校验；文档标注「适配版本」 |
| 合规 | 仅本机、本人多账号、本地文件；不碰凭证、不调云端；README 明确使用边界 |
| 凭证安全 | 绝不读取/复制 `.credentials.json` 等敏感文件 |

---

## 9. 交付计划（v1 一版做全）

**v1 范围 = F1–F8 全部**。但内部按「先稳后扩」的顺序建造，每步都能独立跑通、可验证：

- **阶段 1 — 内核 + 只读**（F1 + F2 + F8）
  - Tauri 脚手架；Rust：`platform` 路径解析、`scanner` 扫描索引、`parser` 解析 JSONL。
  - 前端：账号/对话列表 + 详情渲染。零写盘，先把「全账号可见」立起来。
- **阶段 2 — 续聊 + 搜索**（F5 + F3）
  - `cli-launcher` 起 `claude --resume`；前端搜索过滤。
- **阶段 3 — 迁移（写盘）**（F4 + F7）
  - `migrator`：构造指针、原子写、写前备份、撤销；UI 选目标账号 + 「重启生效」引导。
  - 这是唯一写盘环节，最后做、最严谨（schema 校验 + 备份 + dry-run）。
- **阶段 4 — 导出 + 打磨**（F6 + UI）
  - Markdown 导出；空态/错误态/版本适配提示；打包出 exe。

**后续探索（v1 之后）**：研究「桌面扩展 / MCP」是否能走官方通道实现近实时刷新。

---

## 10. 开源策略

- **建议名**：`ClaudeBridge`（候选：`cc-bridge` / `claude-session-bridge`）。
- **License**：MIT。
- **README 要点**：解决的痛点、一图说清存储架构、跨平台支持、安全声明（非破坏性 / 本地 / 本人多账号）、与现有「只读导出」工具的差异（**唯一能跨号迁移并续聊**）。

---

## 11. 已定决策

1. **项目名**：ClaudeBridge（仓库名 `claude-bridge`）。
2. **技术栈**：Tauri（Rust + Web 前端）。
3. **范围**：v1 一版做全（F1–F8），按 §9 内部顺序建造。
4. **迁移生效方式**：引导用户重启桌面端（已确认唯一可行路径）。

## 12. 开工前置条件（工具链）

- Rust 工具链（`rustup` / `cargo`）。
- Node.js（前端构建，Vite）。
- Tauri CLI（`cargo install tauri-cli` 或 `npm create tauri-app`）。
- Windows 需 WebView2 运行时（Win10/11 通常自带）。
