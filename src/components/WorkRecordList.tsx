import type { ReactNode } from "react";
import { useData } from "../hooks/useData";
import type { WorkRecord } from "../types";

type ManualRecord = Extract<WorkRecord, { source: "manual" }>;

export function WorkRecordStats({ records }: { records: WorkRecord[] }) {
  const completedTasks = new Set(
    records.filter((record) => record.source === "task_completion").map((record) => record.task_id),
  ).size;
  const manualCompleted = records.filter(
    (record) => record.source === "manual" && record.kind === "completed",
  ).length;
  const progress = records.filter(
    (record) => record.source === "manual" && record.kind === "progress",
  ).length;
  return (
    <div className="stats-grid work-record-stats">
      <div><span>完成任务数</span><strong>{completedTasks}</strong></div>
      <div><span>手动完成事项数</span><strong>{manualCompleted}</strong></div>
      <div><span>工作进展数</span><strong>{progress}</strong></div>
    </div>
  );
}

export function WorkRecordList({
  records,
  compact = false,
  actions,
}: {
  records: WorkRecord[];
  compact?: boolean;
  actions?: (record: ManualRecord) => ReactNode;
}) {
  const { data } = useData();
  return (
    <div className={`work-record-list${compact ? " work-record-list-compact" : ""}`}>
      {records.map((record) => {
        const automatic = record.source === "task_completion";
        const label = automatic
          ? "任务完成"
          : record.kind === "completed"
            ? "手动完成"
            : "工作进展";
        return (
          <article className="surface log-card" key={`${record.source}:${record.id}`}>
            <div className="log-card-date">
              <div className="work-record-meta">
                <span>{record.local_date}</span>
                <span className={`work-record-source${automatic || record.kind === "completed" ? " work-record-completed" : ""}`}>
                  {label}
                </span>
              </div>
              {record.source === "manual" && actions && (
                <div className="work-record-actions">{actions(record)}</div>
              )}
            </div>
            <p>{record.content}</p>
            {record.source === "task_completion" && record.reopened_at_utc && (
              <span className="work-record-reopened">之后重新开启过</span>
            )}
            {record.source === "manual" && record.task_id && (
              <span className="linked-task">
                关联 · {data?.tasks.find((task) => task.id === record.task_id)?.title ?? "历史任务"}
              </span>
            )}
          </article>
        );
      })}
    </div>
  );
}
