import type { Report } from "../types";
export function reportText(r: Report) {
  const kind = { day: "每日", week: "每周", month: "每月" }[r.kind] ?? "";
  const lines = (values: string[]) =>
    values.length
      ? values.map((v, i) => `${i + 1}. ${v}`).join("\n")
      : "暂无记录";
  return `DayFlow ${kind}总结\n\n日期：${r.start}${r.start === r.end ? "" : " 至 " + r.end}\n\n一、任务统计\n新增任务：${r.created}\n完成任务：${r.completed}\n完成事件：${r.completion_events}\n期末进行中的任务：${r.in_progress}\n工作日志：${r.logs.length}\n\n二、实际完成事项\n${lines(r.completions.map((e) => e.task_title_snapshot))}\n\n三、工作进展\n${lines(r.logs.map((l) => `${l.local_date}：${l.content}`))}\n\n四、待继续推进的任务（期末状态）\n${lines(r.unfinished.map((t) => t.title))}${r.kind === "week" ? "\n\n五、下周即将到期\n" + lines(r.upcoming.map((t) => `${t.title}（${t.due_date_local}）`)) : ""}${r.kind === "month" ? "\n\n五、已完成任务分类\n" + lines(r.categories.map(([name, count]) => `${name}：${count}`)) : ""}`;
}
