import { useState, type FormEvent } from "react";
import { Plus, Pencil, Trash2, BookOpen, X } from "lucide-react";
import { useData } from "../hooks/useData";
import { api, request } from "../services/api";
import { localDate } from "../services/taskService";
import type { WorkLog } from "../types";
export function WorkLogsView({
  onDirty,
}: {
  onDirty: (dirty: boolean) => void;
}) {
  const { data, run } = useData();
  const [date, setDate] = useState(localDate()),
    [search, setSearch] = useState("");
  const [editing, setEditing] = useState<WorkLog | null>(null),
    [content, setContent] = useState(""),
    [logDate, setLogDate] = useState(localDate()),
    [taskId, setTaskId] = useState(""),
    [busy, setBusy] = useState(false);
  const reset = () => {
    setEditing(null);
    setContent("");
    setTaskId("");
    setLogDate(date || localDate());
    onDirty(false);
  };
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (busy || !content.trim()) return;
    setBusy(true);
    const result = await run(
      () =>
        api.saveLog({
          id: editing?.id,
          expected_updated_at: editing?.updated_at_utc,
          content,
          local_date: logDate,
          task_id: taskId || null,
        }),
      editing ? "日志已更新" : "工作已记录",
    );
    if (result) reset();
    setBusy(false);
  };
  const logs = data!.logs.filter(
    (l) =>
      (!date || l.local_date === date) &&
      l.content.toLocaleLowerCase().includes(search.toLocaleLowerCase()),
  );
  return (
    <div className="view-scroll">
      <div className="page-heading">
        <div>
          <span className="eyebrow">积累每一天的真实进展</span>
          <h1>工作日志</h1>
          <p>计划之外的努力，也值得被记住。</p>
        </div>
        <BookOpen className="page-mark" size={38} />
      </div>
      <div className="logs-layout">
        <section className="surface log-composer">
          <h2>
            {editing ? "编辑工作记录" : "记录今天做过的事"}
            {editing && (
              <button
                className="icon-button"
                title="取消编辑"
                onClick={() => {
                  if (!content || confirm("放弃当前日志编辑？")) reset();
                }}
              >
                <X size={16} />
              </button>
            )}
          </h2>
          <form onSubmit={(e) => void submit(e)}>
            <textarea
              aria-label="工作内容"
              placeholder="例如：帮助同事定位并解决了一个程序异常。"
              rows={5}
              maxLength={20000}
              required
              value={content}
              onChange={(e) => {
                setContent(e.target.value);
                onDirty(!!e.target.value);
              }}
            />
            <div className="form-grid">
              <label>
                所属日期
                <input
                  type="date"
                  min="1900-01-01"
                  max="9999-12-31"
                  required
                  value={logDate}
                  onChange={(e) => {
                    setLogDate(e.target.value);
                    onDirty(true);
                  }}
                />
              </label>
              <label>
                关联任务（可选）
                <select
                  value={taskId}
                  onChange={(e) => {
                    setTaskId(e.target.value);
                    onDirty(true);
                  }}
                >
                  <option value="">不关联任务</option>
                  {data!.tasks
                    .filter((t) => !t.deleted_at_utc || t.id === taskId)
                    .map((t) => (
                      <option key={t.id} value={t.id}>
                        {t.title}
                      </option>
                    ))}
                </select>
              </label>
            </div>
            <button className="primary" disabled={busy || !content.trim()}>
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
                aria-label="查看日志日期"
                type="date"
                value={date}
                onChange={(e) => setDate(e.target.value)}
              />
            </label>
            <button className="text-button" onClick={() => setDate("")}>
              全部日期
            </button>
            <input
              aria-label="搜索日志"
              placeholder="搜索工作内容…"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
          <div className="section-heading">
            <h2>{date ? date + " 的记录" : "所有工作记录"}</h2>
            <span>{logs.length} 条</span>
          </div>
          {logs.length ? (
            logs.map((l) => (
              <article className="surface log-card" key={l.id}>
                <div className="log-card-date">
                  <span>{l.local_date}</span>
                  <div>
                    <button
                      className="icon-button"
                      aria-label="编辑日志"
                      disabled={busy}
                      onClick={() => {
                        if (content && !confirm("放弃当前未保存的日志内容？"))
                          return;
                        setEditing(l);
                        setContent(l.content);
                        setLogDate(l.local_date);
                        setTaskId(l.task_id ?? "");
                        onDirty(false);
                      }}
                    >
                      <Pencil size={15} />
                    </button>
                    <button
                      className="icon-button"
                      aria-label="删除日志"
                      disabled={busy}
                      onClick={() => {
                        if (!confirm("永久删除这条工作日志？")) return;
                        setBusy(true);
                        void run(
                          () => request("delete_log", { id: l.id }),
                          "日志已删除",
                        ).finally(() => setBusy(false));
                      }}
                    >
                      <Trash2 size={15} />
                    </button>
                  </div>
                </div>
                <p>{l.content}</p>
                {l.task_id && (
                  <span className="linked-task">
                    关联 ·{" "}
                    {data!.tasks.find((t) => t.id === l.task_id)?.title ??
                      "历史任务"}
                  </span>
                )}
              </article>
            ))
          ) : (
            <div className="empty-state">
              <BookOpen size={30} />
              <h3>这一天还没有工作记录</h3>
              <p>一段进展、一次帮助，或一个新的发现。</p>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
