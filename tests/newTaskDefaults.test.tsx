import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TaskEditor } from "../src/components/TaskEditor";
import type { Task } from "../src/types";

vi.mock("../src/hooks/useData", () => ({
  useData: () => ({
    data: {
      settings: { default_priority: "medium" },
      categories: [],
      work_records: [],
    },
    run: vi.fn(),
    reportError: vi.fn(),
  }),
}));

function editor(task: Task | null = null) {
  return renderToStaticMarkup(
    <TaskEditor task={task} onClose={() => {}} onSaved={() => {}} onDirty={() => {}} />,
  );
}

function dateInput(html: string) {
  return html.match(/<input[^>]*type="date"[^>]*>/)?.[0];
}

function timeInput(html: string) {
  return html.match(/<input[^>]*type="time"[^>]*>/)?.[0];
}

const existingTask: Task = {
  id: "existing-no-date",
  title: "用户明确不设截止日期的任务",
  description: "",
  status: "todo",
  priority: "medium",
  category_id: null,
  due_date_local: null,
  due_time_local: null,
  due_at_utc: null,
  created_at_utc: "2026-10-16T01:00:00Z",
  updated_at_utc: "2026-10-16T01:00:00Z",
  completed_at_utc: null,
  deleted_at_utc: null,
};

afterEach(() => vi.useRealTimers());

describe("新建任务日期默认值", () => {
  it("完整新建表单使用打开当天的本地日期，时刻保持可选且为空", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 9, 17, 23, 59));
    const html = editor();
    expect(dateInput(html)).toContain('value="2026-10-17"');
    expect(timeInput(html)).toContain('value=""');
    expect(timeInput(html)).not.toContain("disabled");

    vi.setSystemTime(new Date(2026, 9, 18, 0, 1));
    expect(dateInput(editor())).toContain('value="2026-10-18"');
  });

  it("编辑原本无日期的任务时保持无日期，不把今天写入历史任务", () => {
    const html = editor(existingTask);
    expect(dateInput(html)).toContain('value=""');
    expect(timeInput(html)).toContain('value=""');
    expect(timeInput(html)).toContain("disabled");
  });

  it("编辑已安排的任务保留原日期和时刻", () => {
    const html = editor({
      ...existingTask,
      due_date_local: "2026-10-21",
      due_time_local: "15:30",
      due_at_utc: "2026-10-21T07:30:00Z",
    });
    expect(dateInput(html)).toContain('value="2026-10-21"');
    expect(timeInput(html)).toContain('value="15:30"');
    expect(timeInput(html)).not.toContain("disabled");
  });
});
