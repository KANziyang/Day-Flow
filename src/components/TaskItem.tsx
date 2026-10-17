import { Check, Flag, Circle, LoaderCircle, RotateCcw } from "lucide-react";
import { useState } from "react";
import { useData } from "../hooks/useData";
import { api } from "../services/api";
import { dueLabel, isOverdue, localDate } from "../services/taskService";
import type { Task } from "../types";
export function TaskItem({
  task,
  selected,
  onSelect,
  compact = false,
  now,
}: {
  task: Task;
  selected?: boolean;
  onSelect: () => void;
  compact?: boolean;
  now: Date;
}) {
  const { data, run } = useData(),
    [busy, setBusy] = useState(false);
  const completed = task.status === "completed",
    category = data?.categories.find((c) => c.id === task.category_id);
  const toggle = async () => {
    if (busy) return;
    setBusy(true);
    await run(
      () => api.status(task.id, completed ? "todo" : "completed"),
      completed ? "任务已重新开启，历史完成记录已保留" : "已完成，辛苦了",
    );
    setBusy(false);
  };
  return (
    <article
      className={`task-item ${selected ? "selected" : ""} ${completed ? "done" : ""} ${compact ? "compact" : ""} ${data?.settings.completion_animation ? "animate-completion" : ""}`}
    >
      <button
        className="check-button"
        aria-label={`${completed ? "重新开启" : "完成"}：${task.title}`}
        disabled={busy || !!task.deleted_at_utc || task.status === "cancelled"}
        onClick={() => void toggle()}
      >
        {busy ? (
          <LoaderCircle size={18} className="spin" />
        ) : completed ? (
          <Check size={15} />
        ) : task.status === "in_progress" ? (
          <Circle size={18} className="progress-circle" />
        ) : null}
      </button>
      <button className="task-body" onClick={onSelect} title={task.title}>
        <span className="task-title">{task.title}</span>
        <span className="task-meta">
          <span
            className={
              isOverdue(task, now)
                ? "danger-text"
                : task.due_date_local === localDate(now)
                  ? "warning-text"
                  : ""
            }
          >
            {isOverdue(task, now) ? "已逾期 · " : ""}
            {dueLabel(task, now)}
          </span>
          {!compact && category && (
            <span
              className="category-dot"
              style={
                { "--category-color": category.color } as React.CSSProperties
              }
            >
              {category.name}
            </span>
          )}
          {!compact && task.status === "in_progress" && <span>进行中</span>}
          {task.deleted_at_utc && <span>已删除</span>}
        </span>
      </button>
      {task.deleted_at_utc ? (
        <RotateCcw size={14} />
      ) : (
        task.priority === "high" &&
        !completed && (
          <Flag size={14} className="priority-high" aria-label="高重要程度" />
        )
      )}
    </article>
  );
}
