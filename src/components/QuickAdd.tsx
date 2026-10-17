import { Plus, CalendarDays, ArrowUp } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useData } from "../hooks/useData";
import { api } from "../services/api";
import { localDate } from "../services/taskService";
export function QuickAdd({ compact = false }: { compact?: boolean }) {
  const { data, run } = useData();
  const [title, setTitle] = useState(""),
    [date, setDate] = useState<string>(),
    [time, setTime] = useState(""),
    [expand, setExpand] = useState(false),
    [busy, setBusy] = useState(false);
  const selectedDate = date ?? localDate();
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!title.trim() || busy || !data) return;
    const dueDate = date ?? localDate();
    setBusy(true);
    const result = await run(
      () =>
        api.saveTask({
          title,
          description: "",
          status: "todo",
          priority: data.settings.default_priority,
          category_id: null,
          due_date_local: dueDate || null,
          due_time_local: dueDate && time ? time : null,
        }),
      "任务已添加",
    );
    if (result) {
      setTitle("");
      setDate(undefined);
      setTime("");
      setExpand(false);
    }
    setBusy(false);
  };
  return (
    <form
      className={`quick-add ${compact ? "compact-add" : ""}`}
      onSubmit={(e) => void submit(e)}
    >
      <div className="quick-input">
        <Plus size={19} />
        <input
          aria-label="快速添加任务"
          data-quick-add
          placeholder={compact ? "添加一个新任务…" : "添加任务，按 Enter 保存…"}
          value={title}
          maxLength={500}
          onChange={(e) => setTitle(e.target.value)}
        />
        <button
          type="button"
          className={`icon-button ${expand ? "active" : ""}`}
          aria-label="设置截止日期"
          onClick={() => setExpand(!expand)}
        >
          <CalendarDays size={17} />
        </button>
        {title.trim() && (
          <button className="quick-submit" title="添加任务" disabled={busy}>
            <ArrowUp size={16} />
          </button>
        )}
      </div>
      {expand && (
        <div className="quick-date">
          <label>
            截止日期
            <input
              aria-label="快速任务截止日期"
              type="date"
              min="1900-01-01"
              max="9999-12-31"
              value={selectedDate}
              onChange={(e) => {
                setDate(e.target.value);
                if (!e.target.value) setTime("");
              }}
            />
          </label>
          <label>
            时刻（可选）
            <input
              aria-label="快速任务截止时刻"
              type="time"
              disabled={!selectedDate}
              value={time}
              onChange={(e) => setTime(e.target.value)}
            />
          </label>
        </div>
      )}
    </form>
  );
}
