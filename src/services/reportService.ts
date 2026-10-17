import type { Report, Task, WorkRecord } from "../types";

function recordLine(record: WorkRecord) {
  const label = record.source === "task_completion"
    ? "任务完成"
    : record.kind === "completed" ? "手动完成" : "工作进展";
  const reopened = record.source === "task_completion" && record.reopened_at_utc
    ? "（之后重新开启过）" : "";
  return `${record.local_date} · ${label}：${record.content}${reopened}`;
}

function taskLine(task: Task) {
  const due = task.due_date_local
    ? `截止：${task.due_date_local}${task.due_time_local ? " " + task.due_time_local : ""}`
    : "无截止日期";
  return `${task.title}（${due}）`;
}

export function reportText(r: Report) {
  const kind = { day: "每日", week: "每周", month: "每月" }[r.kind] ?? "";
  const nextKind = { day: "明天", week: "下周", month: "下月" }[r.kind] ?? "";
  const lines = (values: string[]) =>
    values.length
      ? values.map((v, i) => `${i + 1}. ${v}`).join("\n")
      : "暂无记录";
  const completed = r.work_records.filter((record) =>
    record.source === "task_completion" || record.kind === "completed");
  const progress = r.work_records.filter((record) =>
    record.source === "manual" && record.kind === "progress");
  const sections = [
    `DayFlow ${kind}总结`,
    `日期：${r.start}${r.start === r.end ? "" : " 至 " + r.end}`,
    "一、实际完成事项\n" + lines(completed.map(recordLine)),
    "二、工作进展\n" + lines(progress.map(recordLine)),
    `三、下一周期需要完成的任务（${nextKind}：${r.next_start} 至 ${r.next_end}）\n` +
      lines(r.upcoming.map(taskLine)),
    "四、未完成的任务\n" + lines(r.unfinished.map(taskLine)),
  ];
  return sections.join("\n\n");
}
