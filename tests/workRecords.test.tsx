import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { WorkRecordList, WorkRecordStats } from "../src/components/WorkRecordList";
import { reportText } from "../src/services/reportService";
import type { Report, Task, WorkRecord } from "../src/types";

vi.mock("../src/hooks/useData", () => ({
  useData: () => ({ data: { tasks: [{ id: "task-1", title: "后来改名的任务" }] } }),
}));

const completion: WorkRecord = {
  source: "task_completion", id: "task:task-1:2026-10-17", event_id: "event-1",
  task_id: "task-1", local_date: "2026-10-17", content: "完成时的任务标题",
  created_at_utc: "2026-10-17T09:00:00Z", reopened_at_utc: "2026-10-18T00:00:00Z",
};
const manual: Extract<WorkRecord, { source: "manual" }> = {
  source: "manual", id: "log-1", task_id: "task-1", kind: "completed",
  local_date: "2026-10-17", content: "临时帮同事排查问题",
  created_at_utc: "2026-10-17T08:00:00Z", updated_at_utc: "2026-10-17T08:00:00Z",
};
const progress: WorkRecord = {
  ...manual, id: "log-2", kind: "progress", content: "整理了一半资料",
};
const records: WorkRecord[] = [completion, manual, progress];
function report(overrides: Partial<Report> = {}): Report {
  return {
    kind: "day", start: "2026-10-17", end: "2026-10-17",
    next_start: "2026-10-18", next_end: "2026-10-18",
    work_records: records, unfinished: [], upcoming: [], ...overrides,
  };
}
function task(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-2", title: "准备评审", description: "", status: "todo", priority: "medium",
    category_id: null, due_date_local: "2026-10-18", due_time_local: "09:30",
    due_at_utc: "2026-10-18T01:30:00Z", created_at_utc: "2026-10-17T08:00:00Z",
    updated_at_utc: "2026-10-17T08:00:00Z", completed_at_utc: null,
    deleted_at_utc: null, ...overrides,
  };
}

describe("统一工作记录与复制总结", () => {
  it("关联同一任务的自动和手动事项独立显示，操作仅提供给手动项", () => {
    const actions = vi.fn((record: typeof manual) => <button>编辑 {record.id}</button>);
    const html = renderToStaticMarkup(<WorkRecordList records={records} actions={actions} />);
    for (const text of ["任务完成", "手动完成", "工作进展", "完成时的任务标题",
      "临时帮同事排查问题", "整理了一半资料", "之后重新开启过"]) {
      expect(html).toContain(text);
    }
    expect(actions.mock.calls.map(([record]) => record.id)).toEqual(["log-1", "log-2"]);
    expect(html.match(/<article /g)).toHaveLength(3);
  });

  it("全部日期的完成任务按任务去重，手动成果和进展不混入任务数", () => {
    const anotherDay: WorkRecord = { ...completion, id: "task:task-1:2026-10-16", local_date: "2026-10-16" };
    const html = renderToStaticMarkup(<WorkRecordStats records={[...records, anotherDay]} />);
    expect(html).toContain("完成任务数</span><strong>1</strong>");
    expect(html).toContain("手动完成事项数</span><strong>1</strong>");
    expect(html).toContain("工作进展数</span><strong>1</strong>");
  });

  it("首页紧凑列表与完整列表使用相同记录、顺序和历史快照", () => {
    const full = renderToStaticMarkup(<WorkRecordList records={records} />);
    const compact = renderToStaticMarkup(<WorkRecordList records={records} compact />);
    expect(compact.replace("work-record-list work-record-list-compact", "work-record-list")).toBe(full);
    expect(full.indexOf("完成时的任务标题")).toBeLessThan(full.indexOf("临时帮同事排查问题"));
  });

  it("复制总结把两种成果放进完成事项，将未完成的进展单独列出", () => {
    const text = reportText(report());
    const accomplishments = text.split("一、实际完成事项\n")[1].split("二、工作进展")[0];
    const advances = text.split("二、工作进展\n")[1].split("三、")[0];
    expect(accomplishments).toContain("任务完成：完成时的任务标题（之后重新开启过）");
    expect(accomplishments).toContain("手动完成：临时帮同事排查问题");
    expect(accomplishments).not.toContain("整理了一半资料");
    expect(advances).toContain("工作进展：整理了一半资料");
    expect(advances).not.toContain("临时帮同事排查问题");
    expect(text).not.toMatch(/工作统计|完成次数|完成任务数|新增任务|期末进行中/);
  });

  it("只有临时成果也能形成完成事项，空周期不产生虚构记录", () => {
    const text = reportText(report({ work_records: [manual] }));
    expect(text).toContain("一、实际完成事项\n1. 2026-10-17 · 手动完成：临时帮同事排查问题");
    const empty = reportText(report({ work_records: [] }));
    expect(empty).toContain("一、实际完成事项\n暂无记录");
    expect(empty.match(/暂无记录/g)).toHaveLength(4);
    expect(empty).not.toContain("完成时的任务标题");
  });

  it.each([
    ["day", "2026-10-17", "2026-10-17", "2026-10-18", "2026-10-18", "明天"],
    ["week", "2026-10-12", "2026-10-18", "2026-10-19", "2026-10-25", "下周"],
    ["month", "2026-10-01", "2026-10-31", "2026-11-01", "2026-11-30", "下月"],
  ])("%s报固定四节，并使用服务端返回的下一周期范围", (kind, start, end, next_start, next_end, label) => {
    const text = reportText(report({ kind, start, end, next_start, next_end,
      upcoming: [task({ due_date_local: next_start, due_at_utc: `${next_start}T01:30:00Z` })] }));
    expect(text.match(/^[一二三四五六]、.+$/gm)).toEqual([
      "一、实际完成事项", "二、工作进展",
      `三、下一周期需要完成的任务（${label}：${next_start} 至 ${next_end}）`,
      "四、未完成的任务",
    ]);
    expect(text).toContain(`日期：${start}${start === end ? "" : " 至 " + end}`);
    expect(text).not.toMatch(/工作统计|趋势|分类|完成次数|新增任务/);
  });

  it("下一周期与未完成任务同时列出截止日期时间、仅日期和无截止日期", () => {
    const upcoming = [task(), task({ id: "task-3", title: "整理材料", due_time_local: null })];
    const unfinishedTasks = [...upcoming,
      task({ id: "task-4", title: "待定事项", due_date_local: null, due_time_local: null, due_at_utc: null }),
      task({ id: "task-5", title: "逾期任务", due_date_local: "2026-10-16", due_time_local: null, due_at_utc: null }),
      task({ id: "task-6", title: "远期任务", due_date_local: "2026-12-01", due_time_local: null, due_at_utc: null })];
    const text = reportText(report({ work_records: [], upcoming, unfinished: unfinishedTasks }));
    const next = text.split("三、下一周期需要完成的任务")[1].split("四、未完成的任务")[0];
    const unfinished = text.split("四、未完成的任务\n")[1];
    for (const section of [next, unfinished]) {
      expect(section).toContain("1. 准备评审（截止：2026-10-18 09:30）");
      expect(section).toContain("2. 整理材料（截止：2026-10-18）");
    }
    expect(unfinished).toContain("3. 待定事项（无截止日期）");
    expect(unfinished).toContain("4. 逾期任务（截止：2026-10-16）");
    expect(unfinished).toContain("5. 远期任务（截止：2026-12-01）");
    expect(next).not.toMatch(/待定事项|逾期任务|远期任务/);
  });

  it("没有本期记录时仍展示下一周期和未完成任务", () => {
    const text = reportText(report({ work_records: [], upcoming: [task()], unfinished: [task({ title: "继续开发" })] }));
    expect(text).toContain("一、实际完成事项\n暂无记录\n\n二、工作进展\n暂无记录");
    expect(text).toContain("准备评审（截止：2026-10-18 09:30）");
    expect(text).toContain("四、未完成的任务\n1. 继续开发（截止：2026-10-18 09:30）");
    expect(text.match(/暂无记录/g)).toHaveLength(2);
  });
});
