import { useEffect, useState, type FormEvent } from "react";
import { Plus, Pencil, Trash2, BookOpen, X } from "lucide-react";
import { useData } from "../hooks/useData";
import { api, request } from "../services/api";
import { localDate } from "../services/taskService";
import { WorkRecordList, WorkRecordStats } from "../components/WorkRecordList";
import type { WorkLog, WorkRecord } from "../types";

interface RecordForm {
  content: string;
  local_date: string;
  task_id: string;
  kind: WorkLog["kind"];
}
function emptyForm(date: string): RecordForm {
  return { content: "", local_date: date, task_id: "", kind: "completed" };
}

export function WorkLogsView({
  onDirty,
}: {
  onDirty: (dirty: boolean) => void;
}) {
  const { data, run } = useData();
  const [date, setDate] = useState(localDate()),
    [search, setSearch] = useState("");
  const [editing, setEditing] = useState<WorkLog | null>(null),
    [base, setBase] = useState(() => emptyForm(localDate())),
    [form, setForm] = useState(base),
    [busy, setBusy] = useState(false);
  const dirty = JSON.stringify(form) !== JSON.stringify(base);
  useEffect(() => {
    onDirty(dirty);
    return () => onDirty(false);
  }, [dirty, onDirty]);
  const set = <K extends keyof RecordForm>(key: K, value: RecordForm[K]) =>
    setForm((previous) => ({ ...previous, [key]: value }));
  const reset = () => {
    const next = emptyForm(date || localDate());
    setEditing(null);
    setBase(next);
    setForm(next);
    onDirty(false);
  };
  const edit = (record: Extract<WorkRecord, { source: "manual" }>) => {
    if (dirty && !confirm("放弃当前未保存的工作记录？")) return;
    const next: RecordForm = {
      content: record.content,
      local_date: record.local_date,
      task_id: record.task_id ?? "",
      kind: record.kind,
    };
    setEditing(record);
    setBase(next);
    setForm(next);
    onDirty(false);
  };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (busy || !form.content.trim()) return;
    setBusy(true);
    const result = await run(
      () => api.saveLog({
        id: editing?.id,
        expected_updated_at: editing?.updated_at_utc,
        content: form.content,
        local_date: form.local_date,
        task_id: form.task_id || null,
        kind: form.kind,
      }),
      editing ? "工作记录已更新" : "工作已记录",
    );
    if (result) reset();
    setBusy(false);
  };
  const records = data!.work_records.filter(
    (record) =>
      (!date || record.local_date === date) &&
      record.content.toLocaleLowerCase().includes(search.toLocaleLowerCase()),
  );
  return (
    <div className="view-scroll">
      <div className="page-heading">
        <div>
          <span className="eyebrow">记住每一天实际做过的事</span>
          <h1>工作记录</h1>
          <p>完成的任务、临时的成果和持续的进展，都在这里。</p>
        </div>
        <BookOpen className="page-mark" size={38} />
      </div>
      <div className="logs-layout">
        <section className="surface log-composer">
          <h2>
            {editing ? "编辑工作记录" : "记录做过的事"}
            {editing && (
              <button
                className="icon-button"
                title="取消编辑"
                disabled={busy}
                onClick={() => {
                  if (!dirty || confirm("放弃当前工作记录编辑？")) reset();
                }}
              >
                <X size={16} />
              </button>
            )}
          </h2>
          <form onSubmit={(event) => void submit(event)}>
            <label>
              记录类型
              <select
                disabled={busy}
                value={form.kind}
                onChange={(event) => set("kind", event.target.value as WorkLog["kind"])}
              >
                <option value="completed">已完成事项</option>
                <option value="progress">工作进展</option>
              </select>
            </label>
            <textarea
              aria-label="工作内容"
              placeholder={form.kind === "completed"
                ? "例如：帮助同事定位并解决了一个程序异常。"
                : "例如：完成方案初稿，等待确认后继续完善。"}
              rows={5}
              maxLength={20000}
              required
              disabled={busy}
              value={form.content}
              onChange={(event) => set("content", event.target.value)}
            />
            <div className="form-grid">
              <label>
                所属日期
                <input
                  type="date"
                  min="1900-01-01"
                  max="9999-12-31"
                  required
                  disabled={busy}
                  value={form.local_date}
                  onChange={(event) => set("local_date", event.target.value)}
                />
              </label>
              <label>
                关联任务（可选）
                <select
                  disabled={busy}
                  value={form.task_id}
                  onChange={(event) => set("task_id", event.target.value)}
                >
                  <option value="">不关联任务</option>
                  {data!.tasks
                    .filter((task) => !task.deleted_at_utc || task.id === form.task_id)
                    .map((task) => (
                      <option key={task.id} value={task.id}>{task.title}</option>
                    ))}
                </select>
              </label>
            </div>
            <button className="primary" disabled={busy || !form.content.trim()}>
              <Plus size={16} />
              {busy ? "保存中…" : editing ? "保存修改" : "保存记录"}
            </button>
          </form>
        </section>
        <section className="log-records">
          <div className="filters">
            <label>
              查看日期
              <input
                aria-label="查看工作记录日期"
                type="date"
                value={date}
                onChange={(event) => setDate(event.target.value)}
              />
            </label>
            <button className="text-button" onClick={() => setDate(localDate())}>今天</button>
            <button className="text-button" onClick={() => setDate("")}>全部日期</button>
            <input
              aria-label="搜索工作记录"
              placeholder="搜索工作内容…"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
            />
          </div>
          <div className="section-heading">
            <h2>{date ? date + " 的记录" : "所有工作记录"}</h2>
            <span>{records.length} 条</span>
          </div>
          <WorkRecordStats records={records} />
          {records.length ? (
            <WorkRecordList
              records={records}
              actions={(record) => <>
                <button
                  className="icon-button"
                  aria-label="编辑工作记录"
                  disabled={busy}
                  onClick={() => edit(record)}
                ><Pencil size={15} /></button>
                <button
                  className="icon-button"
                  aria-label="删除工作记录"
                  disabled={busy}
                  onClick={() => {
                    if (!confirm("永久删除这条工作记录？")) return;
                    setBusy(true);
                    void run(
                      () => request("delete_log", { id: record.id }).then(() => true),
                      "工作记录已删除",
                    ).then((removed) => {
                      if (removed && editing?.id === record.id) reset();
                    }).finally(() => setBusy(false));
                  }}
                ><Trash2 size={15} /></button>
              </>}
            />
          ) : (
            <div className="empty-state">
              <BookOpen size={30} />
              <h3>{search ? "没有匹配的工作记录" : date ? "这一天还没有工作记录" : "还没有工作记录"}</h3>
              <p>{search ? "试试其他关键词或查看日期。" : "完成任务会自动记录，也可以手动记录成果和进展。"}</p>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
