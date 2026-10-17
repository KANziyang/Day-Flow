import { useEffect, useState } from "react";
import {
  Copy,
  BarChart3,
  ChevronLeft,
  ChevronRight,
  Check,
} from "lucide-react";
import { useData } from "../hooks/useData";
import { api, errorText } from "../services/api";
import { localDate } from "../services/taskService";
import { reportText } from "../services/reportService";
import type { Report } from "../types";
export function ReportsView() {
  const { data } = useData(),
    [kind, setKind] = useState("day"),
    [date, setDate] = useState(localDate()),
    [report, setReport] = useState<Report | null>(null),
    [error, setError] = useState(""),
    [loading, setLoading] = useState(true),
    [copied, setCopied] = useState(false);
  useEffect(() => {
    let alive = true;
    setLoading(true);
    setError("");
    if (!date) {
      setLoading(false);
      return;
    }
    void api
      .report(kind, date)
      .then((r) => {
        if (alive) setReport(r);
      })
      .catch((e) => {
        if (alive) setError(errorText(e));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [kind, date, data]);
  useEffect(() => {
    if (!copied) return;
    const id = setTimeout(() => setCopied(false), 3000);
    return () => clearTimeout(id);
  }, [copied]);
  const step = (n: number) => {
    const d = new Date(date + "T12:00:00");
    if (kind === "month") {
      d.setDate(1);
      d.setMonth(d.getMonth() + n);
    } else d.setDate(d.getDate() + n * (kind === "week" ? 7 : 1));
    setDate(localDate(d));
  };
  const points = report
      ? kind === "month"
        ? report.weekly
        : report.daily
      : [],
    max = Math.max(1, ...points.map((p) => p[1]));
  return (
    <div className="view-scroll">
      <div className="page-heading">
        <div>
          <span className="eyebrow">回看过去，安排接下来</span>
          <h1>周期复盘</h1>
          <p>回看真实进展，让下一步更清晰。</p>
        </div>
        <BarChart3 size={38} className="page-mark" />
      </div>
      <div className="report-toolbar">
        <div className="segmented">
          {[
            ["day", "日报"],
            ["week", "周报"],
            ["month", "月报"],
          ].map(([id, label]) => (
            <button
              key={id}
              className={kind === id ? "active" : ""}
              onClick={() => setKind(id)}
            >
              {label}
            </button>
          ))}
        </div>
        <div className="period-picker">
          <button
            className="icon-button"
            aria-label="上一周期"
            onClick={() => step(-1)}
          >
            <ChevronLeft size={17} />
          </button>
          <input
            aria-label="报表日期"
            type="date"
            min="1900-01-01"
            max="9999-12-31"
            value={date}
            onChange={(e) => setDate(e.target.value)}
          />
          <button
            className="icon-button"
            aria-label="下一周期"
            onClick={() => step(1)}
          >
            <ChevronRight size={17} />
          </button>
          <button className="text-button" onClick={() => setDate(localDate())}>
            今天
          </button>
        </div>
      </div>
      {loading ? (
        <div className="loading-state">正在汇总本地记录…</div>
      ) : error ? (
        <div role="alert" className="inline-alert">
          {error}
        </div>
      ) : (
        report && (
          <>
            <div className="stats-grid">
              <div>
                <span>新增任务</span>
                <strong>{report.created}</strong>
              </div>
              <div>
                <span>实际完成任务</span>
                <strong>{report.completed}</strong>
              </div>
              <div>
                <span>工作记录</span>
                <strong>{report.logs.length}</strong>
              </div>
              <div>
                <span>期末进行中</span>
                <strong>{report.in_progress}</strong>
              </div>
            </div>
            {kind !== "day" && (
              <section className="surface trend">
                <div className="section-heading">
                  <h2>{kind === "month" ? "每周完成趋势" : "每日完成任务"}</h2>
                  <span>
                    {report.start} — {report.end}
                  </span>
                </div>
                <div className="bar-chart">
                  {points.map(([d, count]) => (
                    <div
                      className="chart-column"
                      key={d}
                      title={`${d}：${count} 个任务`}
                    >
                      <span>{count}</span>
                      <div className="bar-track">
                        <div style={{ height: `${(count / max) * 100}%` }} />
                      </div>
                      <small>{d.slice(5)}</small>
                    </div>
                  ))}
                </div>
              </section>
            )}
            <section className="surface report-paper">
              <div className="section-heading">
                <h2>
                  {report.start === report.end
                    ? report.start
                    : `${report.start} 至 ${report.end}`}
                </h2>
                <button
                  className="secondary"
                  onClick={() => {
                    void navigator.clipboard
                      .writeText(reportText(report))
                      .then(() => setCopied(true))
                      .catch((e) => setError("复制失败：" + errorText(e)));
                  }}
                >
                  {copied ? <Check size={15} /> : <Copy size={15} />}
                  {copied ? "已复制" : "复制总结"}
                </button>
              </div>
              {report.created === 0 &&
                report.completed === 0 &&
                report.logs.length === 0 && (
                  <div className="report-empty">
                    这个周期还没有新增任务、完成事项或工作日志。
                  </div>
                )}
              <pre>{reportText(report)}</pre>
              <p className="footnote">
                完成任务按周期内任务去重；完成事件保留每一次完成。进行中与待推进事项依据周期末的历史状态。月报分类统计周期内完成的任务，趋势按每天完成任务数汇总。
              </p>
            </section>
          </>
        )
      )}
    </div>
  );
}
