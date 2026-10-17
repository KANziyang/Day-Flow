import { useEffect, useState, type FormEvent } from "react";
import { X, Save, Trash2, RotateCcw, History, Plus } from "lucide-react";
import { useData } from "../hooks/useData";
import { api, request } from "../services/api";
import { localDate } from "../services/taskService";
import {
  priorityLabels,
  statusLabels,
  type Priority,
  type Status,
  type Task,
  type TaskEvent,
  type TaskInput,
} from "../types";
function inputFor(t: Task | null, priority: Priority): TaskInput {
  return {
    id: t?.id,
    title: t?.title ?? "",
    description: t?.description ?? "",
    status: t?.status ?? "todo",
    priority: t?.priority ?? priority,
    category_id: t?.category_id ?? null,
    due_date_local: t?.due_date_local ?? null,
    due_time_local: t?.due_time_local ?? null,
    expected_updated_at: t?.updated_at_utc,
  };
}
const eventLabels: Record<string, string> = {
  created: "创建任务",
  completed: "完成任务",
  reopened: "重新开启",
  status_changed: "状态改变",
  due_changed: "修改截止时间",
  priority_changed: "修改重要程度",
  deleted: "移入回收站",
  restored: "恢复任务",
  updated: "更新任务内容",
};
export function TaskEditor({
  task,
  onClose,
  onSaved,
  onDirty,
}: {
  task: Task | null;
  onClose: () => void;
  onSaved: (id: string) => void;
  onDirty: (dirty: boolean) => void;
}) {
  const { data, run, reportError } = useData();
  const [base, setBase] = useState(() =>
    inputFor(task, data!.settings.default_priority),
  );
  const [form, setForm] = useState(base),
    [busy, setBusy] = useState(false),
    [events, setEvents] = useState<TaskEvent[]>([]),
    [progress, setProgress] = useState("");
  const dirty = JSON.stringify(base) !== JSON.stringify(form) || !!progress.trim();
  useEffect(() => {
    if (task && !dirty) {
      const next = inputFor(task, task.priority);
      setBase(next);
      setForm(next);
    }
  }, [task?.updated_at_utc]);
  useEffect(() => {
    onDirty(dirty);
    return () => onDirty(false);
  }, [dirty, onDirty]);
  useEffect(() => {
    let alive = true;
    if (task)
      void api
        .events(task.id)
        .then((e) => {
          if (alive) setEvents(e.reverse());
        })
        .catch((error) => { if (alive) reportError(error); });
    return () => {
      alive = false;
    };
  }, [task?.id, task?.updated_at_utc, reportError]);
  const set = <K extends keyof TaskInput>(key: K, value: TaskInput[K]) =>
    setForm((old) => ({ ...old, [key]: value }));
  const save = async (e: FormEvent) => {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    const t = await run(() => api.saveTask(form), "任务已保存");
    if (t) {
      const next = inputFor(t, t.priority);
      setBase(next);
      setForm(next);
      onSaved(t.id);
      onDirty(!!progress.trim());
    }
    setBusy(false);
  };
  const remove = async () => {
    if (!task || !confirm("将此任务移入回收站？历史完成记录会保留。")) return;
    setBusy(true);
    const result = await run(
      () => request<boolean>("delete_task", { id: task.id }).then(() => true),
      "已移入回收站",
    );
    setBusy(false);
    if (result) {
      onDirty(false);
      onClose();
    }
  };
  const stale = task && task.updated_at_utc !== base.expected_updated_at;
  const progressLogs = data?.logs.filter((l) => l.task_id === task?.id) ?? [];
  return (
    <aside className="detail-panel">
      <div className="panel-heading">
        <span>{task ? "任务详情" : "新建任务"}</span>
        <button className="icon-button" aria-label="关闭详情" onClick={onClose}>
          <X size={18} />
        </button>
      </div>
      {stale && (
        <div className="inline-alert">
          此任务已在其他窗口更新。
          <button
            onClick={() => {
              if (!dirty || confirm("放弃未保存的修改并重新载入？")) {
                const next = inputFor(task, task.priority);
                setBase(next);
                setForm(next);
              }
            }}
          >
            重新载入
          </button>
        </div>
      )}
      {task?.deleted_at_utc ? (
        <div className="empty-state">
          <Trash2 size={28} />
          <h3>任务在回收站中</h3>
          <p>任务和历史事件均已保留。</p>
          <button
            className="primary"
            disabled={busy}
            onClick={() => {
              setBusy(true);
              void run(
                () => request("restore_task", { id: task.id }),
                "任务已恢复",
              ).finally(() => setBusy(false));
            }}
          >
            <RotateCcw size={16} />
            恢复任务
          </button>
        </div>
      ) : (
        <form className="task-form" onSubmit={(e) => void save(e)}>
          <label className="sr-only" htmlFor="task-title">
            任务标题
          </label>
          <textarea
            id="task-title"
            className="editor-title"
            placeholder="接下来，做些什么？"
            autoFocus={!task}
            rows={2}
            value={form.title}
            maxLength={500}
            required
            onChange={(e) => set("title", e.target.value)}
          />
          <label>
            详细描述
            <textarea
              rows={4}
              placeholder="写下目标、想法或执行步骤…"
              value={form.description}
              maxLength={20000}
              onChange={(e) => set("description", e.target.value)}
            />
          </label>
          <div className="form-grid">
            <label>
              状态
              <select
                value={form.status}
                onChange={(e) => set("status", e.target.value as Status)}
              >
                {Object.entries(statusLabels).map(([v, label]) => (
                  <option key={v} value={v}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
            <label>
              重要程度
              <select
                value={form.priority}
                onChange={(e) => set("priority", e.target.value as Priority)}
              >
                {Object.entries(priorityLabels).map(([v, label]) => (
                  <option key={v} value={v}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
            <label>
              截止日期
              <input
                type="date"
                min="1900-01-01"
                max="9999-12-31"
                value={form.due_date_local ?? ""}
                onChange={(e) => {
                  set("due_date_local", e.target.value || null);
                  if (!e.target.value) set("due_time_local", null);
                }}
              />
            </label>
            <label>
              截止时刻（可选）
              <input
                type="time"
                disabled={!form.due_date_local}
                value={form.due_time_local ?? ""}
                onChange={(e) => set("due_time_local", e.target.value || null)}
              />
            </label>
          </div>
          <label>
            分类
            <select
              value={form.category_id ?? ""}
              onChange={(e) => set("category_id", e.target.value || null)}
            >
              <option value="">未分类</option>
              {data?.categories.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </select>
          </label>
          <div className="form-actions">
            <button className="primary" disabled={busy || !form.title.trim()}>
              <Save size={16} />
              {busy ? "保存中…" : dirty || !task ? "保存任务" : "已保存"}
            </button>
            {task && (
              <button
                type="button"
                className="icon-button danger-text"
                aria-label="删除任务"
                disabled={busy}
                onClick={() => void remove()}
              >
                <Trash2 size={17} />
              </button>
            )}
          </div>
        </form>
      )}
      {task && (
        <div className="detail-history">
          <h3>
            <History size={15} />
            进展与历史
          </h3>
          {!task.deleted_at_utc && (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                if (busy || !progress.trim()) return;
                setBusy(true);
                void run(
                  () =>
                    api.saveLog({
                      task_id: task.id,
                      local_date: localDate(),
                      content: progress,
                    }),
                  "进展已记入今天的工作日志",
                )
                  .then((result) => {
                    if (result) setProgress("");
                  })
                  .finally(() => setBusy(false));
              }}
            >
              <textarea
                aria-label="记录任务进展"
                placeholder="记录今天的进展，无需完成任务…"
                rows={2}
                value={progress}
                maxLength={20000}
                onChange={(e) => setProgress(e.target.value)}
              />
              <button
                className="text-button"
                disabled={busy || !progress.trim()}
              >
                <Plus size={14} />
                记录进展
              </button>
            </form>
          )}
          {progressLogs.slice(0, 10).map((l) => (
            <div className="timeline-entry" key={l.id}>
              <small>{l.local_date} · 工作进展</small>
              <p>{l.content}</p>
            </div>
          ))}
          {events.map((e) => (
            <div className="timeline-entry" key={e.id}>
              <small>
                {new Date(e.created_at_utc).toLocaleString("zh-CN")}
              </small>
              <p>
                {eventLabels[e.event_type] ?? e.event_type}
                {e.new_value && (
                  <span>
                    {" "}
                    ·{" "}
                    {e.event_type === "priority_changed"
                      ? priorityLabels[e.new_value as Priority]
                      : (statusLabels[e.new_value as Status] ?? e.new_value)}
                  </span>
                )}
              </p>
            </div>
          ))}
          <div className="task-timestamps">
            <span>
              创建于 {new Date(task.created_at_utc).toLocaleString("zh-CN")}
            </span>
            {task.completed_at_utc && (
              <span>
                完成于 {new Date(task.completed_at_utc).toLocaleString("zh-CN")}
              </span>
            )}
            <span>
              最近修改 {new Date(task.updated_at_utc).toLocaleString("zh-CN")}
            </span>
            <span className="mono">ID {task.id}</span>
          </div>
        </div>
      )}
    </aside>
  );
}
