import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  IconLink,
  IconRefresh,
  IconSearch,
  IconTerminal,
  IconCli,
  IconMessages,
  IconDownload,
  IconSliders,
  IconCheck,
  IconSun,
  IconMoon,
  IconMonitor,
} from "./icons";
import "./App.css";

interface AccountRef {
  account_id: string;
  org_id: string;
  session_id: string;
  title: string | null;
  is_archived: boolean;
}

interface AccountInfo {
  account_id: string;
  org_id: string;
  session_count: number;
}

interface Conversation {
  cli_session_id: string;
  transcript_path: string;
  project_dir: string;
  cwd: string | null;
  title: string | null;
  model: string | null;
  created_at: number | null;
  last_activity_at: number | null;
  message_count: number;
  size_bytes: number;
  accounts: AccountRef[];
  has_pointer: boolean;
}

interface Message {
  role: string;
  text: string;
  tools: string[];
  timestamp: string | null;
}

interface Toast {
  msg: string;
  undo?: () => void;
}

interface ContentHit {
  cli_session_id: string;
  title: string | null;
  match_count: number;
  snippet: string;
}

type Layout = "split" | "flat";
type SortKey = "recent" | "messages" | "title";
type SortDir = "desc" | "asc";
type GroupBy = "project" | "none";
type DateRange = "all" | "today" | "7d" | "30d";
type Theme = "system" | "light" | "dark";

function fmtTime(ms: number | null): string {
  if (!ms) return "";
  const d = new Date(ms);
  if (isNaN(d.getTime())) return "";
  const sameDay = d.toDateString() === new Date().toDateString();
  return sameDay
    ? d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
    : d.toLocaleDateString([], { month: "2-digit", day: "2-digit" });
}

function fmtSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function acctColor(id: string): string {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) % 360;
  return `hsl(${h}, 52%, 52%)`;
}

/** 从 cwd / 项目目录取一个人类可读的分组标题（取末段） */
function projectLabel(p: string): string {
  if (!p) return "未知项目";
  const parts = p.replace(/[\\/]+$/, "").split(/[\\/]/);
  return parts[parts.length - 1] || p;
}

/** 把命中处包成 <mark>，React 安全（不用 innerHTML） */
function highlight(text: string, q: string): ReactNode {
  const ql = q.trim().toLowerCase();
  if (!ql) return text;
  const lower = text.toLowerCase();
  const out: ReactNode[] = [];
  let i = 0;
  let k = 0;
  while (i <= text.length) {
    const idx = lower.indexOf(ql, i);
    if (idx < 0) {
      out.push(text.slice(i));
      break;
    }
    if (idx > i) out.push(text.slice(i, idx));
    out.push(<mark key={k++}>{text.slice(idx, idx + ql.length)}</mark>);
    i = idx + ql.length;
  }
  return out;
}

function App() {
  const [convos, setConvos] = useState<Conversation[]>([]);
  const [accountsFull, setAccountsFull] = useState<AccountInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [acctFilter, setAcctFilter] = useState<string | null>(null);
  const [selected, setSelected] = useState<Conversation | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [loadingMsgs, setLoadingMsgs] = useState(false);
  const [msgError, setMsgError] = useState<string | null>(null);
  const [layout, setLayout] = useState<Layout>("split");
  const [migrateOpen, setMigrateOpen] = useState(false);
  const [toast, setToast] = useState<Toast | null>(null);
  const migrateRef = useRef<HTMLDivElement>(null);
  const openSeq = useRef(0);

  // 排序 / 分组 / 日期筛选 / 多选
  const [sortKey, setSortKey] = useState<SortKey>("recent");
  const [sortDir, setSortDir] = useState<SortDir>("desc");
  const [groupBy, setGroupBy] = useState<GroupBy>("project");
  const [dateRange, setDateRange] = useState<DateRange>("all");
  const [toolsOpen, setToolsOpen] = useState(false);
  const toolsRef = useRef<HTMLDivElement>(null);
  const [selectMode, setSelectMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [theme, setTheme] = useState<Theme>(
    () => (localStorage.getItem("cb-theme") as Theme) || "system"
  );
  const [searchScope, setSearchScope] = useState<"meta" | "content">("meta");
  const [contentHits, setContentHits] = useState<ContentHit[]>([]);
  const [searching, setSearching] = useState(false);

  async function refresh(): Promise<Conversation[]> {
    setLoading(true);
    setError(null);
    try {
      const [list, accs] = await Promise.all([
        invoke<Conversation[]>("list_conversations"),
        invoke<AccountInfo[]>("list_accounts"),
      ]);
      setConvos(list);
      setAccountsFull(accs);
      // 同步选中项 + 多选集合，避免迁移/撤销后显示过期数据
      setSelected((prev) =>
        prev ? list.find((c) => c.cli_session_id === prev.cli_session_id) ?? prev : prev
      );
      setSelectedIds((prev) => new Set([...prev].filter((id) => list.some((c) => c.cli_session_id === id))));
      return list;
    } catch (e) {
      setError(String(e));
      return [];
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  // 迁移下拉：点击外部关闭
  useEffect(() => {
    if (!migrateOpen) return;
    function onDown(e: MouseEvent) {
      if (migrateRef.current && !migrateRef.current.contains(e.target as Node)) {
        setMigrateOpen(false);
      }
    }
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [migrateOpen]);

  // 排序/筛选浮层：点击外部关闭
  useEffect(() => {
    if (!toolsOpen) return;
    function onDown(e: MouseEvent) {
      if (toolsRef.current && !toolsRef.current.contains(e.target as Node)) {
        setToolsOpen(false);
      }
    }
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [toolsOpen]);

  // toast 自动消失（带撤销的多留一会儿）
  useEffect(() => {
    if (!toast) return;
    const ms = toast.undo ? 9000 : 4500;
    const t = setTimeout(() => setToast(null), ms);
    return () => clearTimeout(t);
  }, [toast]);

  // 主题：system 在运行时解析为 light/dark，写入 <html data-theme>
  useEffect(() => {
    localStorage.setItem("cb-theme", theme);
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const resolved = theme === "system" ? (mq.matches ? "dark" : "light") : theme;
      document.documentElement.dataset.theme = resolved;
    };
    apply();
    if (theme === "system") {
      mq.addEventListener("change", apply);
      return () => mq.removeEventListener("change", apply);
    }
  }, [theme]);

  const cycleTheme = () =>
    setTheme((t) => (t === "system" ? "light" : t === "light" ? "dark" : "system"));

  // 全文搜索：防抖调用后端
  useEffect(() => {
    if (searchScope !== "content") {
      setContentHits([]);
      setSearching(false);
      return;
    }
    const q = query.trim();
    if (q.length < 2) {
      setContentHits([]);
      setSearching(false);
      return;
    }
    setSearching(true);
    const t = setTimeout(async () => {
      try {
        const hits = await invoke<ContentHit[]>("search_content", {
          query: q,
          accountId: acctFilter,
        });
        setContentHits(hits);
      } catch (e) {
        setError(String(e));
      } finally {
        setSearching(false);
      }
    }, 260);
    return () => clearTimeout(t);
  }, [query, searchScope, acctFilter]);

  async function openConvo(c: Conversation) {
    const seq = ++openSeq.current; // 竞态守卫：仅最后一次点击生效
    setSelected(c);
    setMigrateOpen(false);
    setMsgError(null);
    setLoadingMsgs(true);
    setMessages([]);
    try {
      const msgs = await invoke<Message[]>("get_transcript", { path: c.transcript_path });
      if (openSeq.current === seq) setMessages(msgs);
    } catch (e) {
      if (openSeq.current === seq) setMsgError(String(e));
    } finally {
      if (openSeq.current === seq) setLoadingMsgs(false);
    }
  }

  async function resume() {
    if (!selected) return;
    try {
      await invoke("resume_session", {
        cliSessionId: selected.cli_session_id,
        cwd: selected.cwd,
      });
      setToast({ msg: "已在新终端打开，正在续聊…" });
    } catch (e) {
      setToast({ msg: "续聊失败：" + e });
    }
  }

  async function exportMd() {
    if (!selected) return;
    try {
      const path = await invoke<string>("export_markdown", {
        transcriptPath: selected.transcript_path,
        title: selected.title,
      });
      setToast({ msg: "已导出 Markdown：" + path });
    } catch (e) {
      setToast({ msg: "导出失败：" + e });
    }
  }

  async function doMigrate(a: AccountInfo) {
    if (!selected || !selected.cwd) return;
    setMigrateOpen(false);
    try {
      const res = await invoke<{ session_id: string; file_path: string }>("migrate_session", {
        cliSessionId: selected.cli_session_id,
        cwd: selected.cwd,
        title: selected.title,
        model: selected.model,
        targetAccountId: a.account_id,
        targetOrgId: a.org_id,
      });
      setToast({
        msg: `已迁移到 ${a.account_id.slice(0, 8)} · 重启桌面端后生效`,
        undo: async () => {
          try {
            await invoke("undo_migrate", { filePath: res.file_path });
            setToast({ msg: "已撤销这次迁移" });
            refresh();
          } catch (e) {
            setToast({ msg: "撤销失败：" + e });
          }
        },
      });
      refresh();
    } catch (e) {
      setToast({ msg: "迁移失败：" + e });
    }
  }

  function toggleSelect(id: string) {
    setSelectedIds((prev) => {
      const n = new Set(prev);
      if (n.has(id)) n.delete(id);
      else n.add(id);
      return n;
    });
  }

  const accounts = useMemo(() => {
    const m = new Map<string, number>();
    for (const c of convos)
      for (const a of c.accounts) m.set(a.account_id, (m.get(a.account_id) ?? 0) + 1);
    return Array.from(m.entries()).sort((a, b) => b[1] - a[1]);
  }, [convos]);

  const dateCutoff = useMemo(() => {
    if (dateRange === "all") return 0;
    const now = Date.now();
    const day = 86400000;
    if (dateRange === "today") {
      const d = new Date();
      d.setHours(0, 0, 0, 0);
      return d.getTime();
    }
    if (dateRange === "7d") return now - 7 * day;
    return now - 30 * day;
  }, [dateRange]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = convos.filter((c) => {
      if (acctFilter && !c.accounts.some((a) => a.account_id === acctFilter)) return false;
      if (dateCutoff) {
        const ts = c.last_activity_at ?? c.created_at ?? 0;
        if (ts < dateCutoff) return false;
      }
      if (!q) return true;
      return (
        (c.title ?? "").toLowerCase().includes(q) ||
        (c.cwd ?? "").toLowerCase().includes(q) ||
        c.project_dir.toLowerCase().includes(q) ||
        c.cli_session_id.toLowerCase().includes(q)
      );
    });
    const dir = sortDir === "asc" ? 1 : -1;
    return list.sort((a, b) => {
      if (sortKey === "title") return dir * (a.title ?? "").localeCompare(b.title ?? "");
      if (sortKey === "messages") return dir * (a.message_count - b.message_count);
      return dir * ((a.last_activity_at ?? 0) - (b.last_activity_at ?? 0));
    });
  }, [convos, query, acctFilter, dateCutoff, sortKey, sortDir]);

  const groups = useMemo(() => {
    if (groupBy === "none") return [{ key: "__all", label: "", items: filtered }];
    const map = new Map<string, { label: string; items: Conversation[] }>();
    for (const c of filtered) {
      const key = c.cwd || c.project_dir || "未知项目";
      if (!map.has(key)) map.set(key, { label: projectLabel(key), items: [] });
      map.get(key)!.items.push(c);
    }
    const maxTs = (items: Conversation[]) =>
      items.reduce((m, c) => Math.max(m, c.last_activity_at ?? 0), 0);
    return Array.from(map.entries())
      .map(([key, v]) => ({ key, label: v.label, items: v.items }))
      .sort((a, b) => maxTs(b.items) - maxTs(a.items));
  }, [filtered, groupBy]);

  async function batchExport() {
    const items = filtered.filter((c) => selectedIds.has(c.cli_session_id));
    if (items.length === 0) return;
    let ok = 0;
    let fail = 0;
    for (const c of items) {
      try {
        await invoke<string>("export_markdown", {
          transcriptPath: c.transcript_path,
          title: c.title,
        });
        ok++;
      } catch {
        fail++;
      }
    }
    setToast({ msg: `已导出 ${ok} 条 Markdown${fail ? ` · ${fail} 条失败` : ""}（在 Downloads）` });
  }

  const filtersActive =
    sortKey !== "recent" || sortDir !== "desc" || groupBy !== "project" || dateRange !== "all";

  return (
    <div className="app">
      <aside className="sidebar">
        <header className="side-head">
          <div className="brand">
            <IconLink size={17} className="brand-mark" />
            <span className="brand-name">ClaudeBridge</span>
          </div>
          <div className="head-actions">
            <button
              className="icon-btn"
              onClick={cycleTheme}
              title={`主题：${
                theme === "system" ? "跟随系统" : theme === "light" ? "浅色" : "深色"
              }（点击切换）`}
              aria-label="切换主题"
            >
              {theme === "system" ? (
                <IconMonitor size={15} />
              ) : theme === "light" ? (
                <IconSun size={15} />
              ) : (
                <IconMoon size={15} />
              )}
            </button>

            <div className="tools-wrap" ref={toolsRef}>
              <button
                className={`icon-btn ${filtersActive ? "active" : ""}`}
                onClick={() => setToolsOpen((v) => !v)}
                title="排序与筛选"
                aria-label="排序与筛选"
              >
                <IconSliders size={15} />
              </button>
              {toolsOpen && (
                <div className="menu tools-menu">
                  <div className="menu-title">排序</div>
                  <div className="seg sm">
                    <button className={sortKey === "recent" ? "on" : ""} onClick={() => setSortKey("recent")}>
                      最近
                    </button>
                    <button className={sortKey === "messages" ? "on" : ""} onClick={() => setSortKey("messages")}>
                      条数
                    </button>
                    <button className={sortKey === "title" ? "on" : ""} onClick={() => setSortKey("title")}>
                      标题
                    </button>
                  </div>
                  <button className="tools-row" onClick={() => setSortDir((d) => (d === "desc" ? "asc" : "desc"))}>
                    方向 <span>{sortDir === "desc" ? "降序 ↓" : "升序 ↑"}</span>
                  </button>

                  <div className="menu-title">分组</div>
                  <div className="seg sm">
                    <button className={groupBy === "project" ? "on" : ""} onClick={() => setGroupBy("project")}>
                      按项目
                    </button>
                    <button className={groupBy === "none" ? "on" : ""} onClick={() => setGroupBy("none")}>
                      不分组
                    </button>
                  </div>

                  <div className="menu-title">时间范围</div>
                  <div className="seg sm wrap">
                    <button className={dateRange === "all" ? "on" : ""} onClick={() => setDateRange("all")}>
                      全部
                    </button>
                    <button className={dateRange === "today" ? "on" : ""} onClick={() => setDateRange("today")}>
                      今天
                    </button>
                    <button className={dateRange === "7d" ? "on" : ""} onClick={() => setDateRange("7d")}>
                      近 7 天
                    </button>
                    <button className={dateRange === "30d" ? "on" : ""} onClick={() => setDateRange("30d")}>
                      近 30 天
                    </button>
                  </div>
                </div>
              )}
            </div>

            <button
              className={`icon-btn ${selectMode ? "active" : ""}`}
              onClick={() => {
                setSelectMode((v) => !v);
                setSelectedIds(new Set());
              }}
              title="多选"
              aria-label="多选"
            >
              <IconCheck size={15} />
            </button>

            <button className="icon-btn" onClick={refresh} title="重新扫描" aria-label="刷新">
              <IconRefresh size={15} />
            </button>
          </div>
        </header>

        <div className="search-wrap">
          <IconSearch size={15} className="search-icon" />
          <input
            className="search"
            placeholder={searchScope === "content" ? "搜索对话正文…" : "搜索标题、路径或 ID"}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <div className="search-scope">
          <div className="seg sm">
            <button
              className={searchScope === "meta" ? "on" : ""}
              onClick={() => setSearchScope("meta")}
            >
              标题/路径
            </button>
            <button
              className={searchScope === "content" ? "on" : ""}
              onClick={() => setSearchScope("content")}
            >
              全文
            </button>
          </div>
        </div>

        <div className="chips">
          <button
            className={`chip ${acctFilter === null ? "on" : ""}`}
            onClick={() => setAcctFilter(null)}
          >
            全部 <span className="chip-n">{convos.length}</span>
          </button>
          {accounts.map(([id, n]) => (
            <button
              key={id}
              className={`chip ${acctFilter === id ? "on" : ""}`}
              onClick={() => setAcctFilter(acctFilter === id ? null : id)}
            >
              <span className="dot" style={{ background: acctColor(id) }} />
              {id.slice(0, 6)} <span className="chip-n">{n}</span>
            </button>
          ))}
        </div>

        <div className="list">
          {error && <div className="err">{error}</div>}

          {searchScope === "content" && (
            <>
              {searching && <div className="hint">搜索正文中…</div>}
              {!searching && query.trim().length < 2 && (
                <div className="hint">输入至少 2 个字搜索正文</div>
              )}
              {!searching && query.trim().length >= 2 && contentHits.length === 0 && (
                <div className="hint">正文里没找到匹配</div>
              )}
              {contentHits.map((h) => (
                <button
                  key={h.cli_session_id}
                  className={`item ${selected?.cli_session_id === h.cli_session_id ? "sel" : ""}`}
                  onClick={() => {
                    const full = convos.find((c) => c.cli_session_id === h.cli_session_id);
                    if (full) openConvo(full);
                  }}
                >
                  <div className="item-row">
                    <span className="item-title">{h.title || "未命名对话"}</span>
                    <span className="item-count">{h.match_count} 处</span>
                  </div>
                  <div className="snippet">{highlight(h.snippet, query)}</div>
                </button>
              ))}
            </>
          )}

          {searchScope !== "content" && loading && <div className="hint">扫描中…</div>}
          {searchScope !== "content" && !loading && filtered.length === 0 && (
            <div className="hint">没有匹配的对话</div>
          )}
          {searchScope !== "content" &&
            groups.map((g) => (
            <div key={g.key} className="group">
              {groupBy === "project" && g.items.length > 0 && (
                <div className="group-head">
                  <span className="group-label" title={g.key}>
                    {g.label}
                  </span>
                  <span className="group-n">{g.items.length}</span>
                </div>
              )}
              {g.items.map((c) => {
                const checked = selectedIds.has(c.cli_session_id);
                return (
                  <button
                    key={c.cli_session_id}
                    className={`item ${selected?.cli_session_id === c.cli_session_id ? "sel" : ""} ${
                      selectMode ? "selmode" : ""
                    } ${checked ? "checked" : ""}`}
                    onClick={() => (selectMode ? toggleSelect(c.cli_session_id) : openConvo(c))}
                  >
                    {selectMode && (
                      <span className={`item-check ${checked ? "on" : ""}`}>
                        {checked && <IconCheck size={12} />}
                      </span>
                    )}
                    <div className="item-row">
                      <span className="item-title">{c.title || "未命名对话"}</span>
                      <span className="item-time">{fmtTime(c.last_activity_at)}</span>
                    </div>
                    <div className="item-sub">
                      <span className="item-path">{c.cwd || c.project_dir}</span>
                    </div>
                    <div className="item-foot">
                      <span className="dots">
                        {c.accounts.length === 0 ? (
                          <span className="tag-cli">仅 CLI</span>
                        ) : (
                          c.accounts.map((a, i) => (
                            <span
                              key={i}
                              className="dot"
                              style={{ background: acctColor(a.account_id) }}
                              title={`${a.account_id}/${a.org_id}`}
                            />
                          ))
                        )}
                      </span>
                      <span className="item-count">{c.message_count} 条</span>
                    </div>
                  </button>
                );
              })}
            </div>
          ))}
        </div>

        {selectMode && selectedIds.size > 0 && (
          <div className="batch-bar">
            <span className="batch-n">已选 {selectedIds.size}</span>
            <button className="act" onClick={batchExport}>
              <IconDownload size={13} /> 导出 MD
            </button>
            <button className="batch-clear" onClick={() => setSelectedIds(new Set())}>
              清空
            </button>
          </div>
        )}
      </aside>

      <main className="detail">
        {!selected ? (
          <div className="empty">
            <IconMessages size={28} className="empty-icon" />
            <h2>选择一条对话</h2>
            <p>本机所有账号的记录都在左侧，无需切号即可查看、搜索、续聊或迁移。</p>
          </div>
        ) : (
          <>
            <header className="detail-head">
              <div className="detail-head-top">
                <h2>{selected.title || "未命名对话"}</h2>
                <div className="seg" role="group" aria-label="视图切换">
                  <button className={layout === "split" ? "on" : ""} onClick={() => setLayout("split")}>
                    左右
                  </button>
                  <button className={layout === "flat" ? "on" : ""} onClick={() => setLayout("flat")}>
                    平铺
                  </button>
                </div>
              </div>

              <div className="detail-sub">
                <code className="chip-code">{selected.cwd || selected.project_dir}</code>
                {selected.model && <span className="tag">{selected.model}</span>}
                <span className="tag">{fmtSize(selected.size_bytes)}</span>
                <span className="tag mono">{selected.cli_session_id.slice(0, 8)}</span>
              </div>

              <div className="detail-actions">
                <button
                  className="act"
                  onClick={resume}
                  disabled={!selected.cwd}
                  title={selected.cwd ? "在命令行续聊（无视账号）" : "缺少 cwd，无法续聊"}
                >
                  <IconCli size={14} /> 命令行续聊
                </button>

                <button className="act" onClick={exportMd} title="导出为 Markdown 文件">
                  <IconDownload size={14} /> 导出 MD
                </button>

                <div className="migrate-wrap" ref={migrateRef}>
                  <button
                    className="act primary"
                    onClick={() => setMigrateOpen((v) => !v)}
                    disabled={!selected.cwd}
                    title={selected.cwd ? "复制到另一个账号" : "缺少 cwd，无法迁移"}
                  >
                    <IconLink size={14} /> 迁移到账号…
                  </button>
                  {migrateOpen && (
                    <div className="menu">
                      <div className="menu-title">复制到哪个账号（重启桌面端后生效）</div>
                      {accountsFull.map((a) => {
                        const has = selected.accounts.some(
                          (x) => x.account_id === a.account_id && x.org_id === a.org_id
                        );
                        return (
                          <button
                            key={a.account_id + a.org_id}
                            disabled={has}
                            onClick={() => doMigrate(a)}
                          >
                            <span className="dot" style={{ background: acctColor(a.account_id) }} />
                            <span className="m-id">{a.account_id.slice(0, 8)}</span>
                            {has ? (
                              <span className="m-count">已有</span>
                            ) : (
                              <span className="m-count">{a.session_count} 条</span>
                            )}
                          </button>
                        );
                      })}
                    </div>
                  )}
                </div>
              </div>
            </header>

            <div className={`messages ${layout}`}>
              {loadingMsgs && <div className="hint">加载中…</div>}
              {!loadingMsgs && msgError && <div className="err">{msgError}</div>}
              {!loadingMsgs &&
                messages.map((m, i) => (
                  <div key={i} className={`msg ${m.role}`}>
                    <div className="msg-role">
                      <span className={`role-dot ${m.role}`} />
                      {m.role === "user" ? "你" : "Claude"}
                    </div>
                    <div className="msg-bubble">
                      {m.text && (
                        <div className="msg-text">
                          {searchScope === "content" && query.trim()
                            ? highlight(m.text, query)
                            : m.text}
                        </div>
                      )}
                      {m.tools.length > 0 && (
                        <div className="msg-tools">
                          {m.tools.map((t, j) => (
                            <span key={j} className="tool">
                              <IconTerminal size={12} />
                              {t}
                            </span>
                          ))}
                        </div>
                      )}
                    </div>
                  </div>
                ))}
              {!loadingMsgs && !msgError && messages.length === 0 && (
                <div className="hint">这条对话没有可显示的消息。</div>
              )}
            </div>
          </>
        )}
      </main>

      {toast && (
        <div className="toast">
          <span>{toast.msg}</span>
          {toast.undo && <button onClick={toast.undo}>撤销</button>}
          <button onClick={() => setToast(null)}>关闭</button>
        </div>
      )}
    </div>
  );
}

export default App;
