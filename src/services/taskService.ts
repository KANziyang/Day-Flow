import type { Task } from "../types";
export function localDate(date = new Date()): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}
export function isActive(t: Task) {
  return (
    !t.deleted_at_utc && (t.status === "todo" || t.status === "in_progress")
  );
}
export function isOverdue(t: Task, now = new Date()) {
  if (!isActive(t) || !t.due_date_local) return false;
  if (!t.due_time_local) return t.due_date_local < localDate(now);
  return t.due_at_utc
    ? Date.parse(t.due_at_utc) < now.getTime()
    : new Date(`${t.due_date_local}T${t.due_time_local}`).getTime() <
        now.getTime();
}
export function sortTasks(tasks: Task[], now = new Date()) {
  const today = localDate(now);
  const bucket = (t: Task) =>
    isOverdue(t, now)
      ? 0
      : t.due_date_local === today
        ? 1
        : t.due_date_local
          ? 2
          : 3;
  const rank = { high: 0, medium: 1, low: 2 };
  return [...tasks].sort(
    (a, b) =>
      bucket(a) - bucket(b) ||
      (a.due_date_local ?? "9999-12-31").localeCompare(
        b.due_date_local ?? "9999-12-31",
      ) ||
      (a.due_time_local ?? "23:59:59").localeCompare(
        b.due_time_local ?? "23:59:59",
      ) ||
      rank[a.priority] - rank[b.priority] ||
      a.created_at_utc.localeCompare(b.created_at_utc),
  );
}
export function dueLabel(t: Task, now = new Date()) {
  if (!t.due_date_local) return "无截止日期";
  const today = localDate(now),
    tomorrow = new Date(now);
  tomorrow.setDate(tomorrow.getDate() + 1);
  const date =
    t.due_date_local === today
      ? "今天"
      : t.due_date_local === localDate(tomorrow)
        ? "明天"
        : t.due_date_local.slice(5).replace("-", "月") + "日";
  return `${date}${t.due_time_local ? " " + t.due_time_local : ""}`;
}
export function isTodayTask(t: Task, now = new Date()) {
  return (
    !t.deleted_at_utc &&
    ((isActive(t) &&
      (isOverdue(t, now) || t.due_date_local === localDate(now))) ||
      (t.status === "completed" &&
        !!t.completed_at_utc &&
        localDate(new Date(t.completed_at_utc)) === localDate(now)))
  );
}
