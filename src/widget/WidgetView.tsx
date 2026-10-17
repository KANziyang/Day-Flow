import { useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Maximize2, CheckCheck, Leaf, Minus, Minimize2 } from "lucide-react";
import { useClock, useData } from "../hooks/useData";
import { native, windowAction } from "../services/api";
import {
  isActive,
  isOverdue,
  localDate,
  sortTasks,
} from "../services/taskService";
import { TaskItem } from "../components/TaskItem";
import { QuickAdd } from "../components/QuickAdd";
export function WidgetView() {
  const { data, loading, error, run, refresh, notice, dismiss } = useData(),
    now = useClock();
  const [expanded, setExpanded] = useState(false);
  const today = localDate(now),
    active = data ? sortTasks(data.tasks.filter(isActive), now) : [];
  const done =
    data?.tasks
      .filter(
        (t) =>
          !t.deleted_at_utc &&
          t.status === "completed" &&
          t.completed_at_utc &&
          localDate(new Date(t.completed_at_utc)) === today,
      )
      .sort((a, b) => b.completed_at_utc!.localeCompare(a.completed_at_utc!)) ??
    [];
  const todayPending = active.filter(
    (t) => t.due_date_local === today || isOverdue(t, now),
  ).length;
  const total = todayPending + done.length,
    percent = total ? Math.round((done.length / total) * 100) : 0;
  return (
    <main
      className="widget-shell"
      style={
        {
          "--widget-opacity": data?.settings.widget_opacity ?? 1,
        } as React.CSSProperties
      }
    >
      <header
        className="widget-header"
        onMouseDown={(e) => {
          if (
            e.button === 0 &&
            !(e.target as HTMLElement).closest("button") &&
            native
          )
            void getCurrentWindow().startDragging();
        }}
      >
        <span className="widget-brand">
          <Leaf size={17} />
          DayFlow
        </span>
        <span className="widget-date">
          {now.toLocaleDateString("zh-CN", { month: "long", day: "numeric" })}
        </span>
        <button
          className="icon-button"
          aria-label="展开完整窗口"
          title="打开 DayFlow"
          onClick={() => {
            void run(() => windowAction("main"));
          }}
        >
          <Maximize2 size={15} />
        </button>
        <button
          className="icon-button"
          aria-label="收起为悬浮球"
          title="收起为悬浮球"
          onClick={() => { void run(() => windowAction("collapse_widget")); }}
        >
          <Minimize2 size={15} />
        </button>
        <button
          className="icon-button widget-hide"
          aria-label="隐藏小组件"
          title="隐藏（可从托盘恢复）"
          onClick={() => {
            void run(() => windowAction("hide_widget"));
          }}
        >
          <Minus size={14} />
        </button>
      </header>
      {error && (
        <div role="alert" className="widget-error">
          {error}
          <button
            className="text-button"
            onClick={() => {
              dismiss();
              void refresh();
            }}
          >
            重试
          </button>
        </div>
      )}
      {loading ? (
        <div className="loading-state">正在打开本地记录…</div>
      ) : (
        data && (
          <>
            <div className="widget-progress">
              <div>
                <h1>今天，稳步向前</h1>
                <span>
                  {done.length}
                  <small> / {total}</small>
                </span>
              </div>
              <div className="progress-track">
                <div style={{ width: `${percent}%` }} />
              </div>
            </div>
            <div className="widget-list">
              <div className="widget-section-label">
                待办事项 <span>{active.length}</span>
              </div>
              <div id="widget-pending-tasks">
                {active.length ? (
                  active
                    .slice(0, expanded ? undefined : data.settings.widget_count)
                    .map((t) => (
                      <TaskItem
                        key={t.id}
                        task={t}
                        compact
                        now={now}
                        onSelect={() => {
                          void run(() => windowAction("main", t.id));
                        }}
                      />
                    ))
                ) : (
                  <div className="widget-empty">
                    <CheckCheck size={26} />
                    <p>暂时没有待办事项</p>
                    <small>从一件小事开始吧。</small>
                  </div>
                )}
              </div>
              {active.length > data.settings.widget_count && (
                <button
                  className="widget-more"
                  aria-expanded={expanded}
                  aria-controls="widget-pending-tasks"
                  onClick={() => setExpanded((value) => !value)}
                >
                  {expanded
                    ? "收起"
                    : `查看其余 ${active.length - data.settings.widget_count} 项`}
                </button>
              )}
              {data.settings.show_completed && done.length > 0 && (
                <div className="widget-done">
                  <div className="widget-section-label">今日完成</div>
                  {done.slice(0, 2).map((t) => (
                    <TaskItem
                      key={t.id}
                      task={t}
                      compact
                      now={now}
                      onSelect={() => {
                        void run(() => windowAction("main", t.id));
                      }}
                    />
                  ))}
                </div>
              )}
            </div>
            <footer className="widget-footer">
              <QuickAdd compact />
              <div className="widget-status">
                {notice || "始终置顶"}
              </div>
            </footer>
          </>
        )
      )}
    </main>
  );
}
