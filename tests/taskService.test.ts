import { describe, expect, it } from "vitest";
import {
  dueLabel,
  isOverdue,
  isTodayTask,
  localDate,
  sortTasks,
} from "../src/services/taskService";
import type { Task } from "../src/types";
const now = new Date(2026, 9, 17, 12, 0, 0);
function task(
  id: string,
  date: string | null,
  time: string | null = null,
  priority: Task["priority"] = "medium",
): Task {
  return {
    id,
    title: id,
    description: "",
    status: "todo",
    priority,
    category_id: null,
    due_date_local: date,
    due_time_local: time,
    due_at_utc: time && date ? new Date(`${date}T${time}`).toISOString() : null,
    created_at_utc: "2026-10-01T00:00:00Z",
    updated_at_utc: "",
    completed_at_utc: null,
    deleted_at_utc: null,
  };
}
describe("小组件与主窗口共享的排序规则", () => {
  it("逾期、今天、未来、无日期，且同一时刻按重要程度", () => {
    const values = [
      task("无日期", null),
      task("下周", "2026-10-24"),
      task("明天", "2026-10-18"),
      task("今天低", "2026-10-17", "18:00", "low"),
      task("昨天", "2026-10-16"),
      task("今天高", "2026-10-17", "18:00", "high"),
      task("今天中", "2026-10-17", "18:00"),
      task("今早", "2026-10-17", "09:00"),
    ];
    expect(sortTasks(values, now).map((t) => t.id)).toEqual([
      "昨天",
      "今早",
      "今天高",
      "今天中",
      "今天低",
      "明天",
      "下周",
      "无日期",
    ]);
  });
  it("仅日期任务当天结束前不会逾期", () => {
    const t = task("仅日期", "2026-10-17");
    expect(isOverdue(t, new Date(2026, 9, 17, 23, 59, 59))).toBe(false);
    expect(isOverdue(t, new Date(2026, 9, 18, 0, 0))).toBe(true);
  });
  it("无时刻任务排在同日具体时刻之后", () =>
    expect(
      sortTasks(
        [task("仅日期", "2026-10-17"), task("有时间", "2026-10-17", "23:59")],
        now,
      ).map((t) => t.id),
    ).toEqual(["有时间", "仅日期"]));
  it("相同截止时间和重要性按创建时间", () => {
    const a = task("先", null),
      b = { ...task("后", null), created_at_utc: "2026-10-02T00:00:00Z" };
    expect(sortTasks([b, a], now).map((t) => t.id)).toEqual(["先", "后"]);
  });
  it("完成与取消不标记逾期，今日包含当天实际完成", () => {
    const t = {
      ...task("完成", "2026-10-01"),
      status: "completed" as const,
      completed_at_utc: now.toISOString(),
    };
    expect(isOverdue(t, now)).toBe(false);
    expect(isTodayTask(t, now)).toBe(true);
    expect(isOverdue({ ...t, status: "cancelled" }, now)).toBe(false);
  });
  it("格式与明天提示使用本地日期", () => {
    expect(localDate(now)).toBe("2026-10-17");
    expect(dueLabel(task("明天", "2026-10-18", "18:00"), now)).toBe(
      "明天 18:00",
    );
  });
});
