import { useEffect, useState } from "react";
import {
  Settings2,
  FolderOpen,
  Database,
  Download,
  RotateCcw,
  Plus,
  Save,
} from "lucide-react";
import { useData } from "../hooks/useData";
import { APP_VERSION_LABEL } from "../services/appInfo";
import {
  autostart,
  dataAction,
  errorText,
  request,
  windowAction,
} from "../services/api";
import {
  priorityLabels,
  type Category,
  type Priority,
  type Settings,
} from "../types";
function Toggle({
  label,
  hint,
  checked,
  disabled,
  change,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  disabled?: boolean;
  change: (checked: boolean) => void;
}) {
  return (
    <label className="setting-row">
      <span>
        {label}
        {hint && <small>{hint}</small>}
      </span>
      <input
        className="switch"
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(e) => change(e.target.checked)}
      />
    </label>
  );
}
export function SettingsView() {
  const { data, run } = useData(),
    s = data!.settings;
  const [busy, setBusy] = useState(false),
    [auto, setAuto] = useState<boolean | null>(null),
    [autoError, setAutoError] = useState("");
  useEffect(() => {
    let alive = true;
    void autostart()
      .then((v) => {
        if (alive) setAuto(v);
      })
      .catch((e) => {
        if (alive) setAutoError(errorText(e));
      });
    return () => {
      alive = false;
    };
  }, []);
  const save = <K extends keyof Settings>(key: K, value: Settings[K]) => {
    if (busy) return;
    setBusy(true);
    void run(
      () => request("save_settings", { key, value }),
      "设置已保存",
    ).finally(() => setBusy(false));
  };
  const changeWidget = (action: "widget" | "show_ball" | "hide_widget") => {
    if (busy) return;
    setBusy(true);
    void run(() => windowAction(action)).finally(() => setBusy(false));
  };
  const dataOp = (action: string) => {
    if (busy) return;
    setBusy(true);
    void run(async () => {
      const result = await dataAction(action);
      if (action !== "open_dir") setResult(result);
      return result;
    }).finally(() => setBusy(false));
  };
  const [result, setResult] = useState("");
  return (
    <div className="view-scroll settings-view">
      <div className="page-heading">
        <div>
          <span className="eyebrow">你的 DayFlow，你的节奏</span>
          <h1>设置</h1>
          <p>按自己的节奏，安排 DayFlow。</p>
        </div>
        <Settings2 size={38} className="page-mark" />
      </div>
      <section className="surface setting-section">
        <h2>小组件</h2>
        <div className="setting-row">
          <span>
            始终置顶
            <small>小组件显示在其他普通窗口上方</small>
          </span>
        </div>
        <label className="setting-row">
          <span>最多显示任务</span>
          <select
            value={s.widget_count}
            disabled={busy}
            onChange={(e) => save("widget_count", +e.target.value as 3 | 5 | 8)}
          >
            {[3, 5, 8].map((v) => (
              <option value={v} key={v}>
                {v} 条
              </option>
            ))}
          </select>
        </label>
        <label className="setting-row">
          <span>
            背景不透明度<small>保留文字清晰度</small>
          </span>
          <input
            aria-label="小组件背景不透明度"
            type="range"
            min="0.35"
            max="1"
            step="0.05"
            defaultValue={s.widget_opacity}
            disabled={busy}
            onPointerUp={(e) =>
              save("widget_opacity", +(e.target as HTMLInputElement).value)
            }
            onKeyUp={(e) =>
              save("widget_opacity", +(e.target as HTMLInputElement).value)
            }
          />
        </label>
        <Toggle
          label="记住窗口位置和大小"
          checked={s.remember_position}
          disabled={busy}
          change={(v) => save("remember_position", v)}
        />
        <Toggle
          label="显示最近完成事项"
          checked={s.show_completed}
          disabled={busy}
          change={(v) => save("show_completed", v)}
        />
        <Toggle
          label="显示小组件"
          hint="打开后展开任务小窗口"
          checked={data!.widget_presentation === "expanded"}
          disabled={busy}
          change={(visible) => changeWidget(visible ? "widget" : "hide_widget")}
        />
        <Toggle
          label="显示悬浮球"
          hint="打开后收起为悬浮球"
          checked={data!.widget_presentation === "ball"}
          disabled={busy}
          change={(visible) => changeWidget(visible ? "show_ball" : "hide_widget")}
        />
      </section>
      <section className="surface setting-section">
        <h2>外观与使用</h2>
        <label className="setting-row">
          <span>主题</span>
          <select
            value={s.theme}
            disabled={busy}
            onChange={(e) => save("theme", e.target.value as Settings["theme"])}
          >
            <option value="system">跟随系统</option>
            <option value="light">浅色</option>
            <option value="dark">深色</option>
          </select>
        </label>
        <Toggle
          label="开机启动"
          hint={autoError || "登录 Windows 后恢复小组件上次的展开、悬浮球或隐藏状态"}
          checked={auto ?? false}
          disabled={busy || auto === null}
          change={(v) => {
            setBusy(true);
            void run(() => autostart(v), "开机启动已更新")
              .then((result) => {
                if (result !== undefined) setAuto(result);
              })
              .finally(() => setBusy(false));
          }}
        />
        <Toggle
          label="关闭主窗口后继续后台运行"
          hint="可从系统托盘彻底退出"
          checked={s.close_to_tray}
          disabled={busy}
          change={(v) => save("close_to_tray", v)}
        />
        <Toggle
          label="任务完成动画"
          checked={s.completion_animation}
          disabled={busy}
          change={(v) => save("completion_animation", v)}
        />
        <label className="setting-row">
          <span>默认重要程度</span>
          <select
            value={s.default_priority}
            disabled={busy}
            onChange={(e) =>
              save("default_priority", e.target.value as Priority)
            }
          >
            {Object.entries(priorityLabels).map(([v, label]) => (
              <option value={v} key={v}>
                {label}
              </option>
            ))}
          </select>
        </label>
      </section>
      <section className="surface setting-section">
        <h2>到期提醒</h2>
        <Toggle
          label="Windows 系统通知"
          hint="每个截止时间只提醒一次；仅日期任务按下方时刻提醒"
          checked={s.notifications}
          disabled={busy}
          change={(v) => save("notifications", v)}
        />
        <label className="setting-row">
          <span>仅日期任务的提醒时刻</span>
          <input
            type="time"
            required
            value={s.date_reminder_time}
            disabled={busy}
            onChange={(e) => {
              if (e.target.value) save("date_reminder_time", e.target.value);
            }}
          />
        </label>
        <div className="setting-row">
          <span>
            勿扰时段<small>开始与结束相同表示不启用勿扰</small>
          </span>
          <div className="time-pair">
            <input
              aria-label="勿扰开始"
              type="time"
              value={s.quiet_start}
              disabled={busy}
              onChange={(e) => {
                if (e.target.value) save("quiet_start", e.target.value);
              }}
            />
            <span>至</span>
            <input
              aria-label="勿扰结束"
              type="time"
              value={s.quiet_end}
              disabled={busy}
              onChange={(e) => {
                if (e.target.value) save("quiet_end", e.target.value);
              }}
            />
          </div>
        </div>
        <p className="footnote">
          休眠恢复后会重新检查。系统通知的显示还受 Windows
          通知权限与专注助手控制。
        </p>
      </section>
      <section className="surface setting-section">
        <h2>任务分类</h2>
        <div className="category-editor-list">
          {data!.categories.map((c) => (
            <CategoryEditor key={c.id} category={c} />
          ))}
          <CategoryEditor />
        </div>
      </section>
      <section className="surface setting-section">
        <h2>本地数据</h2>
        <p className="data-path">{data!.data_dir}</p>
        <div className="data-buttons">
          <button
            className="secondary"
            disabled={busy}
            onClick={() => dataOp("open_dir")}
          >
            <FolderOpen size={16} />
            打开数据目录
          </button>
          <button
            className="secondary"
            disabled={busy}
            onClick={() => dataOp("export")}
          >
            <Download size={16} />
            导出 JSON
          </button>
          <button
            className="secondary"
            disabled={busy}
            onClick={() => dataOp("backup")}
          >
            <Database size={16} />
            备份数据库
          </button>
          <button
            className="secondary"
            disabled={busy}
            onClick={() => dataOp("restore")}
          >
            <RotateCcw size={16} />
            从备份恢复
          </button>
        </div>
        <p className="footnote">
          每天首次启动自动创建一致性备份。恢复前会校验数据库并创建原数据安全备份，确认后才替换。
        </p>
        {result && (
          <div className="data-result" role="status">
            {result}
          </div>
        )}
      </section>
      <p className="about">DayFlow {APP_VERSION_LABEL} · 数据留在本机，离线也能专注。</p>
    </div>
  );
}
function CategoryEditor({ category }: { category?: Category }) {
  const { run } = useData(),
    [name, setName] = useState(category?.name ?? ""),
    [color, setColor] = useState(category?.color ?? "#4184ad"),
    [busy, setBusy] = useState(false);
  return (
    <form
      className="category-editor"
      onSubmit={(e) => {
        e.preventDefault();
        if (busy || !name.trim()) return;
        setBusy(true);
        void run(
          () => request("save_category", { id: category?.id, name, color }),
          "分类已保存",
        )
          .then(() => {
            if (!category) setName("");
          })
          .finally(() => setBusy(false));
      }}
    >
      <input
        type="color"
        aria-label="分类颜色"
        value={color}
        onChange={(e) => setColor(e.target.value)}
      />
      <input
        aria-label={category ? `分类名称：${category.name}` : "新分类名称"}
        placeholder="新建分类…"
        value={name}
        maxLength={30}
        required
        onChange={(e) => setName(e.target.value)}
      />
      <button
        className="icon-button"
        title={category ? "保存分类" : "添加分类"}
        disabled={busy || !name.trim()}
      >
        {category ? <Save size={16} /> : <Plus size={16} />}
      </button>
    </form>
  );
}
