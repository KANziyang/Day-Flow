import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  Sun,
  ListTodo,
  BookOpen,
  ChartNoAxesCombined,
  Settings2,
  Leaf,
  Plus,
  Search,
  ArrowUpRight,
  CheckCheck,
  Clock3,
  Flag,
  X,
  PanelTop,
  Power,
  Trash2,
} from "lucide-react";
import { useClock, useData } from "./hooks/useData";
import { native, windowAction } from "./services/api";
import {
  isActive,
  isOverdue,
  isTodayTask,
  localDate,
  sortTasks,
} from "./services/taskService";
import { TaskItem } from "./components/TaskItem";
import { TaskEditor } from "./components/TaskEditor";
import { QuickAdd } from "./components/QuickAdd";
import { WorkLogsView } from "./views/WorkLogsView";
import { ReportsView } from "./views/ReportsView";
import { SettingsView } from "./views/SettingsView";
import { priorityLabels, statusLabels } from "./types";
type Page = "today" | "tasks" | "logs" | "reports" | "settings";
const nav = [
  { id: "today", label: "今日任务", icon: Sun },
  { id: "tasks", label: "全部任务", icon: ListTodo },
  { id: "logs", label: "工作日志", icon: BookOpen },
  { id: "reports", label: "周期复盘", icon: ChartNoAxesCombined },
  { id: "settings", label: "设置", icon: Settings2 },
] as const;
export function App() {
  const { data, loading, error, notice, dismiss, refresh, run } = useData(),
    now = useClock();
  const [page, setPage] = useState<Page>("today"),
    [selected, setSelected] = useState<string | null>(null),
    [creating, setCreating] = useState(false),
    [dirty, setDirty] = useState(false);
  const [search, setSearch] = useState(""),
    [status, setStatus] = useState("active"),
    [priority, setPriority] = useState(""),
    [category, setCategory] = useState("");
  const canLeave = useCallback(
    () => !dirty || confirm("有尚未保存的修改，确定放弃？"),
    [dirty],
  );
  const navigate = (next: Page) => {
    if (page === next || !canLeave()) return;
    setDirty(false);
    setPage(next);
    setSelected(null);
    setCreating(false);
  };
  const select = useCallback(
    (id: string) => {
      if (!canLeave()) return;
      setDirty(false);
      setCreating(false);
      setSelected(id);
    },
    [canLeave],
  );
  const create = () => {
    if (!canLeave()) return;
    setPage("tasks");
    setDirty(false);
    setSelected(null);
    setCreating(true);
  };
  useEffect(() => {
    if (!native) return;
    const subscription = listen<string>("open-task", (e) => {
      if (!canLeave()) return;
      setDirty(false);
      setPage("tasks");
      setStatus("all");
      setSearch("");
      setCategory("");
      setPriority("");
      setCreating(false);
      setSelected(e.payload);
    });
    return () => {
      void subscription.then((fn) => fn());
    };
  }, [canLeave]);
  useEffect(() => {
    if (!native) return;
    const sub = listen("main-close-request", () => {
      if (canLeave()) void windowAction("quit");
    });
    return () => {
      void sub.then((fn) => fn());
    };
  }, [canLeave]);
  useEffect(() => {
    const unload = (e: BeforeUnloadEvent) => {
      if (dirty) {
        e.preventDefault();
        e.returnValue = "";
      }
    };
    const keys = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        document.querySelector<HTMLInputElement>("[data-search]")?.focus();
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "n") {
        e.preventDefault();
        document.querySelector<HTMLInputElement>("[data-quick-add]")?.focus();
      }
      if (e.key === "Escape" && canLeave()) {
        setSelected(null);
        setCreating(false);
        setDirty(false);
      }
    };
    window.addEventListener("beforeunload", unload);
    window.addEventListener("keydown", keys);
    return () => {
      window.removeEventListener("beforeunload", unload);
      window.removeEventListener("keydown", keys);
    };
  }, [dirty, canLeave]);
  const completedIds = new Set(data?.completion_events.filter(([,at]) => localDate(new Date(at)) === localDate(now)).map(([id]) => id));
  const active = data?.tasks.filter(isActive) ?? [],
    todayTasks = data?.tasks.filter((t) => isTodayTask(t, now) || !t.deleted_at_utc && completedIds.has(t.id)) ?? [];
  const doneToday = todayTasks.filter((t) => t.status === "completed").length,
    overdue = active.filter((t) => isOverdue(t, now)).length;
  const todayLogs =
    data?.logs.filter((l) => l.local_date === localDate(now)) ?? [];
  let visible =
    page === "today"
      ? todayTasks
      : (data?.tasks.filter((t) =>
          status === "deleted"
            ? !!t.deleted_at_utc
            : !t.deleted_at_utc &&
              (status === "all" ||
                (status === "active" ? isActive(t) : t.status === status)),
        ) ?? []);
  visible = visible.filter(
    (t) =>
      `${t.title} ${t.description}`
        .toLocaleLowerCase()
        .includes(search.toLocaleLowerCase()) &&
      (!priority || t.priority === priority) &&
      (!category || t.category_id === category),
  );
  const pending = sortTasks(
      visible.filter((t) => t.status !== "completed"),
      now,
    ),
    completed = visible
      .filter((t) => t.status === "completed")
      .sort((a, b) =>
        (b.completed_at_utc ?? "").localeCompare(a.completed_at_utc ?? ""),
      );
  const current = data?.tasks.find((t) => t.id === selected) ?? null;
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-symbol">
            <Leaf size={21} />
          </span>
          <div>
            DayFlow<small>让每一天有迹可循</small>
          </div>
        </div>
        <div className="workspace-label">个人空间</div>
        <nav>
          {nav.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              className={page === id ? "active" : ""}
              onClick={() => navigate(id)}
            >
              <Icon size={18} />
              <span>{label}</span>
              {id === "today" &&
                active.filter(
                  (t) =>
                    t.due_date_local === localDate(now) || isOverdue(t, now),
                ).length > 0 && (
                  <small>
                    {
                      active.filter(
                        (t) =>
                          t.due_date_local === localDate(now) ||
                          isOverdue(t, now),
                      ).length
                    }
                  </small>
                )}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <button
            onClick={() => {
              void run(() => windowAction("widget"));
            }}
          >
            <PanelTop size={17} />
            显示桌面小组件
            <ArrowUpRight size={14} />
          </button>
          <div className="local-indicator">
            <i />
            本地存储 · 离线可用
          </div>
          <button
            className="exit-button"
            onClick={() => {
              if (canLeave()) void run(() => windowAction("quit"));
            }}
          >
            <Power size={14} />
            退出应用
          </button>
        </div>
      </aside>
      <main className="main-shell">
        <header className="topbar">
          <span>
            我的工作台{" "}
            <span className="breadcrumb">
              / {nav.find((n) => n.id === page)?.label}
            </span>
          </span>
          <div className="topbar-right">
            <label className="global-search">
              <Search size={16} />
              <input
                data-search
                aria-label="搜索任务"
                placeholder="搜索任务"
                value={search}
                onChange={(e) => {
                  if (page !== "tasks" && page !== "today") {
                    if (!canLeave()) return;
                    setPage("tasks");
                    setDirty(false);
                  }
                  setSearch(e.target.value);
                }}
              />
              <kbd>Ctrl K</kbd>
            </label>
            <button className="primary" disabled={!data} onClick={create}>
              <Plus size={16} />
              新建任务
            </button>
          </div>
        </header>
        {error && (
          <div role="alert" className="error-banner">
            <span>{error}</span>
            <button
              onClick={() => {
                void refresh();
              }}
            >
              重新加载
            </button>
            <button
              className="icon-button"
              aria-label="关闭错误提示"
              onClick={dismiss}
            >
              <X size={15} />
            </button>
          </div>
        )}
        {notice && (
          <div className="toast" role="status">
            <CheckCheck size={16} />
            {notice}
          </div>
        )}
        {loading && !data ? (
          <div className="loading-state">正在打开 DayFlow 本地数据库…</div>
        ) : !data ? (
          <div className="empty-state">
            <h2>暂时无法打开数据</h2>
            <p>{error}</p>
            <button
              className="primary"
              onClick={() => {
                void refresh();
              }}
            >
              重新连接
            </button>
          </div>
        ) : page === "logs" ? (
          <WorkLogsView onDirty={setDirty} />
        ) : page === "reports" ? (
          <ReportsView />
        ) : page === "settings" ? (
          <SettingsView />
        ) : (
          <div className="task-workspace">
            <div className="task-main">
              <div className="page-heading">
                <div>
                  <span className="eyebrow">
                    {page === "today"
                      ? now.toLocaleDateString("zh-CN", {
                          month: "long",
                          day: "numeric",
                          weekday: "long",
                        })
                      : "把想法变成行动"}
                  </span>
                  <h1>
                    {page === "today" ? "专注今天，从容前行。" : "全部任务"}
                  </h1>
                  <p>
                    {page === "today"
                      ? "把重要的事做好，也给生活留一点余白。"
                      : "整理每一个想法，推进每一件重要的事。"}
                  </p>
                </div>
                <Sun size={36} className="page-mark" />
              </div>
              {page === "today" && (
                <>
                  <div className="stats-grid today-stats">
                    <div>
                      <span>
                        <CheckCheck size={14} />
                        今日曾完成
                      </span>
                      <strong>
                        {completedIds.size}
                        <small> 项</small>
                      </strong>
                    </div>
                    <div>
                      <span>
                        <Clock3 size={14} />
                        待推进
                      </span>
                      <strong>
                        {todayTasks.filter(isActive).length}
                        <small> 项</small>
                      </strong>
                    </div>
                    <div>
                      <span>
                        <Flag size={14} />
                        逾期任务
                      </span>
                      <strong className={overdue ? "warning-text" : ""}>
                        {overdue}
                        <small> 项</small>
                      </strong>
                    </div>
                  </div>
                  <div className="today-progress">
                    <span>今日任务进度</span>
                    <div className="progress-track">
                      <div
                        style={{
                          width: `${todayTasks.length ? (doneToday / todayTasks.length) * 100 : 0}%`,
                        }}
                      />
                    </div>
                    <span>
                      {doneToday} / {todayTasks.length}
                    </span>
                  </div>
                </>
              )}
              {page === "tasks" && (
                <div className="filters">
                  <select
                    aria-label="状态筛选"
                    value={status}
                    onChange={(e) => setStatus(e.target.value)}
                  >
                    <option value="active">待推进</option>
                    <option value="all">全部状态</option>
                    {Object.entries(statusLabels).map(([v, l]) => (
                      <option key={v} value={v}>
                        {l}
                      </option>
                    ))}
                    <option value="deleted">回收站</option>
                  </select>
                  <select
                    aria-label="重要程度筛选"
                    value={priority}
                    onChange={(e) => setPriority(e.target.value)}
                  >
                    <option value="">全部重要程度</option>
                    {Object.entries(priorityLabels).map(([v, l]) => (
                      <option key={v} value={v}>
                        {l}重要程度
                      </option>
                    ))}
                  </select>
                  <select
                    aria-label="分类筛选"
                    value={category}
                    onChange={(e) => setCategory(e.target.value)}
                  >
                    <option value="">全部分类</option>
                    {data.categories.map((c) => (
                      <option key={c.id} value={c.id}>
                        {c.name}
                      </option>
                    ))}
                  </select>
                  <span className="sort-hint">按截止时间排序</span>
                </div>
              )}
              <section className="surface task-list-card">
                <div className="section-heading">
                  <h2>
                    {status === "deleted" && page === "tasks" ? (
                      <>
                        <Trash2 size={16} />
                        回收站
                      </>
                    ) : page === "today" ? (
                      "今日待办"
                    ) : (
                      "任务清单"
                    )}
                    <span className="count-pill">{pending.length}</span>
                  </h2>
                  <span>截止时间优先</span>
                </div>
                {pending.length ? (
                  pending.map((t) => (
                    <TaskItem
                      key={t.id}
                      task={t}
                      selected={selected === t.id}
                      now={now}
                      onSelect={() => select(t.id)}
                    />
                  ))
                ) : (
                  <div className="empty-state task-empty">
                    <CheckCheck size={30} />
                    <h3>
                      {search
                        ? "没有匹配的任务"
                        : page === "today"
                          ? "今天的安排，留给你来书写"
                          : "这里暂时没有任务"}
                    </h3>
                    <p>
                      {page === "today"
                        ? "添加一个今天到期的任务，或直接记录已做的事。"
                        : "只写一个标题，也能开始。"}
                    </p>
                  </div>
                )}
                <QuickAdd />
              </section>
              {completed.length > 0 && (
                <section className="surface completed-list">
                  <div className="section-heading">
                    <h2>
                      已完成
                      <span className="count-pill">{completed.length}</span>
                    </h2>
                  </div>
                  {completed.map((t) => (
                    <TaskItem
                      key={t.id}
                      task={t}
                      now={now}
                      selected={selected === t.id}
                      onSelect={() => select(t.id)}
                    />
                  ))}
                </section>
              )}
              {page === "today" && (
                <section className="surface today-journal">
                  <div className="section-heading">
                    <h2>
                      <BookOpen size={17} />
                      今日工作记录
                    </h2>
                    <button
                      className="text-button"
                      onClick={() => navigate("logs")}
                    >
                      记录今天做过的事
                      <ArrowUpRight size={15} />
                    </button>
                  </div>
                  {todayLogs.length ? (
                    todayLogs.map((l) => <p key={l.id}>{l.content}</p>)
                  ) : (
                    <p className="muted">
                      还没有工作记录。计划之外的成果，也可以在这里留下。
                    </p>
                  )}
                </section>
              )}
            </div>
            {(selected || creating) && (
              <TaskEditor
                key={creating ? "new" : selected}
                task={current}
                onDirty={setDirty}
                onSaved={(id) => {
                  setSelected(id);
                  setCreating(false);
                  setDirty(false);
                }}
                onClose={() => {
                  if (canLeave()) {
                    setSelected(null);
                    setCreating(false);
                    setDirty(false);
                  }
                }}
              />
            )}
          </div>
        )}
        <footer className="statusbar">
          <span>
            <i />
            所有已保存内容保存在本机
          </span>
          <span>
            {dirty ? "有未保存修改" : "Ctrl N 快速添加 · Ctrl K 搜索"}
          </span>
        </footer>
      </main>
    </div>
  );
}
