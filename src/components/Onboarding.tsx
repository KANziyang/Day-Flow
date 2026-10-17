import { useEffect, useRef, useState } from "react";
import {
  Leaf, Plus, ListTodo, BookOpen, ChartNoAxesCombined, PanelTop,
  Settings2, CheckCheck, ChevronLeft, ChevronRight, Check,
} from "lucide-react";
import { errorText } from "../services/api";

const steps = [
  {
    label: "认识 DayFlow", icon: Leaf,
    title: "让每一天，有迹可循。",
    description: "欢迎使用 DayFlow。用几分钟认识从安排任务、记录工作到回顾成果的完整流程。",
    preview: ["安排要做的事", "记录实际进展", "回顾一天的成果"],
    points: [
      ["先安排，再行动", "在「今日任务」关注今天到期和已经逾期的待办，在「全部任务」管理所有安排。"],
      ["做过的事，都能留下", "完成任务会自动形成记录，临时完成的工作和持续推进的进展也可以单独记录。"],
      ["按自己的节奏开始", "数据保存在本机，离线也能使用。任何一步都可以选择「跳过教程」，直接开始使用。"],
    ],
  },
  {
    label: "添加与安排任务", icon: Plus,
    title: "把一个想法，变成一件待办。",
    description: "先写下要做什么，再按需要补充日期、分类和细节。",
    preview: ["准备本周工作计划", "截止日期：今天 · 时刻可选", "分类：工作 · 重要程度：中"],
    points: [
      ["最快的添加方式", "在任务列表底部输入标题，按 Enter 保存。主窗口和小组件都能快速添加。"],
      ["补充安排与细节", "点击右上角「新建任务」，填写描述、状态、重要程度、分类和截止时间；点击已有任务的标题可编辑详情，修改后保存。"],
      ["日期可以自己决定", "新任务默认今天，时刻留空。点击快捷输入旁的日历图标可修改或清空日期；没有日期时无需设置时刻。"],
    ],
  },
  {
    label: "推进与整理任务", icon: ListTodo,
    title: "看清轻重缓急，稳步推进。",
    description: "安排会变化，完成的成果会被保留。",
    preview: ["今日任务：今天到期与逾期待办", "全部任务：状态、分类、重要程度筛选", "完成任务 → 自动进入工作记录"],
    points: [
      ["选择合适的任务视图", "「今日任务」聚焦今天需要推进的事；未来任务和未设日期的任务可以在「全部任务」找到。列表优先按截止时间排列。"],
      ["勾选完成，也能重新开启", "点击任务左侧的圆圈完成任务，再次点击可重新开启。重开、改名或删除任务都不会抹除已经产生的完成历史。"],
      ["查找和恢复任务", "顶部可以搜索标题与描述；「全部任务」支持状态、分类和重要程度筛选。删除的任务进入回收站，可在状态筛选中选择「回收站」恢复。"],
    ],
  },
  {
    label: "记录实际工作", icon: BookOpen,
    title: "计划之外的成果，也值得记录。",
    description: "打开左侧「工作记录」，按日期保存真正做过的事。",
    preview: ["任务完成 · 从勾选的任务自动生成", "手动完成 · 临时解决了一个问题", "工作进展 · 完成方案初稿，待确认"],
    points: [
      ["完成任务会自动留痕", "任务完成后归入实际完成当天的工作记录，自动记录保留完成时的标题，无需再手动重复录入。"],
      ["记录临时完成的工作", "在「工作记录」选择「已完成事项」，输入工作内容并保存。可以补录历史日期，关联任务是可选的，也不会改变任务状态。"],
      ["记录还在推进的事", "尚未完成的工作选择「工作进展」，也可在任务详情中「记录进展」。手动记录支持编辑；日期筛选和搜索可帮助查找历史。"],
    ],
  },
  {
    label: "查看周期复盘", icon: ChartNoAxesCombined,
    title: "回看做成的事，安排下一步。",
    description: "打开左侧「周期复盘」，把分散的任务和记录汇成一份总结。",
    preview: ["日报 / 周报 / 月报 · 选择要回看的日期", "实际完成事项 · 工作进展", "下一周期需完成的任务 · 未完成任务"],
    points: [
      ["选择周期和日期", "切换日报、周报或月报，再选择日期；两侧箭头可切换上一周期或下一周期。周报与月报会自动归属所选日期所在的周和月。"],
      ["看懂四个部分", "报告分别整理实际完成事项、工作进展、下一周期需要完成的任务，以及周期结束时的未完成任务，便于同时回顾成果和安排后续。"],
      ["复制总结", "点击「复制总结」复制当前报告正文，可粘贴到自己的文档或消息中。内容来自已经保存的任务和工作记录。"],
    ],
  },
  {
    label: "小组件与悬浮球", icon: PanelTop,
    title: "需要时展开，专注时收起。",
    description: "置顶小组件让待办随手可见，悬浮球让桌面保持轻巧。",
    preview: ["小组件：查看、添加和完成任务", "收起为悬浮球 → 点击球展开", "设置：显示小组件 / 显示悬浮球"],
    points: [
      ["收起与展开", "点击小组件顶部的「收起为悬浮球」按钮，点击球再展开。收起和隐藏会保留本次运行中尚未提交的输入，展开位置跟随球。"],
      ["移动和查看更多任务", "拖动小组件顶部或悬浮球可移动位置，拖动小组件边缘可缩放。点击「查看其余 N 项」可在小组件内展开更多待办，列表支持滚动。"],
      ["按需要显示或隐藏", "设置里的「显示小组件」「显示悬浮球」两个开关互斥，全部关闭时完全隐藏；也可从托盘恢复。球的右键菜单提供展开与隐藏，小组件顶部还有打开主窗口和隐藏按钮。"],
    ],
  },
  {
    label: "设置与数据备份", icon: Settings2,
    title: "让 DayFlow，适合你的习惯。",
    description: "在左侧「设置」中调整外观、使用方式和数据管理。",
    preview: ["主题：浅色 / 深色 / 跟随系统", "通知与免打扰 · 开机启动", "本地数据：备份数据库 / 从备份恢复"],
    points: [
      ["调整外观和小组件", "可以修改主题、小组件显示条数、背景不透明度，以及是否记住位置和大小；分类也在设置中管理。"],
      ["按需启用提醒与开机启动", "到期通知需要在设置中开启，可设置免打扰时段和仅有日期的任务提醒时刻。开启开机启动后，小组件会恢复上次的展开、球或隐藏状态。"],
      ["保存自己的数据副本", "每天首次启动会自动备份，也能手动「备份数据库」。恢复时选择数据库备份并确认，应用会先备份当前数据。JSON 导出可用于留存数据，「打开数据目录」可查看文件位置。"],
    ],
  },
  {
    label: "开始使用", icon: CheckCheck,
    title: "从今天的一件小事开始。",
    description: "现在，你已经认识了 DayFlow 的完整使用流程。",
    preview: ["添加今天要做的一件事", "记录一项已经完成的工作", "在周期复盘里回看真实成果"],
    points: [
      ["从一个小目标开始", "点击「开始使用」回到工作台。你可以先添加一条任务，也可以直接去「工作记录」记下今天已经做过的事。"],
      ["记住三个快捷键", "Ctrl+N 聚焦快速添加，Ctrl+K 聚焦搜索，Esc 关闭任务详情；有未保存的修改时会先询问。"],
      ["关闭窗口与彻底退出", "默认关闭主窗口后仍在后台运行，可从系统托盘重新打开。要彻底退出，请使用左下角「退出应用」或托盘中的「退出 DayFlow」。"],
    ],
  },
] as const;

export function Onboarding({ onFinish }: { onFinish: () => Promise<void> }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const finishing = useRef(false);
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const current = steps[step], Icon = current.icon;
  useEffect(() => {
    const element = dialog.current!;
    element.showModal();
    return () => element.close();
  }, []);
  useEffect(() => {
    if (content.current) content.current.scrollTop = 0;
    heading.current?.focus();
  }, [step]);
  const finish = async () => {
    if (finishing.current) return;
    finishing.current = true;
    setBusy(true);
    setError("");
    try { await onFinish(); }
    catch (e) { setError(errorText(e)); }
    finally { finishing.current = false; setBusy(false); }
  };
  return (
    <dialog
      ref={dialog}
      className="onboarding-dialog"
      aria-labelledby="onboarding-title"
      aria-describedby="onboarding-description"
      onCancel={(e) => { e.preventDefault(); void finish(); }}
      onKeyDown={(e) => {
        if (e.key === "Tab") {
          const buttons = [...dialog.current!.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
          const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
          if (index < 0 || (e.shiftKey && index === 0) || (!e.shiftKey && index === buttons.length - 1)) {
            e.preventDefault();
            buttons[e.shiftKey ? buttons.length - 1 : 0]?.focus();
          }
        }
        if ((e.ctrlKey || e.metaKey) && ["n", "k"].includes(e.key.toLowerCase())) e.preventDefault();
        e.stopPropagation();
      }}
    >
      <header className="onboarding-header">
        <span><Leaf size={20} /> DayFlow <small>新手教程</small></span>
        <span className="onboarding-counter" aria-live="polite">{step + 1} / {steps.length}</span>
      </header>
      <div className="onboarding-progress" role="progressbar" aria-label="教程进度" aria-valuemin={0} aria-valuemax={steps.length} aria-valuenow={step + 1}>
        <span style={{ width: `${((step + 1) / steps.length) * 100}%` }} />
      </div>
      <div className="onboarding-layout">
        <aside className="onboarding-outline" aria-label="教程目录">
          <p>从安排到复盘</p>
          <ol>
            {steps.map((item, index) => (
              <li key={item.label} aria-current={index === step ? "step" : undefined} className={index < step ? "is-read" : ""}>
                <span>{index < step ? <Check size={12} /> : String(index + 1).padStart(2, "0")}</span>
                {item.label}
              </li>
            ))}
          </ol>
        </aside>
        <div className="onboarding-content" ref={content}>
          <span className="onboarding-step-icon"><Icon size={25} /></span>
          <h1 id="onboarding-title" tabIndex={-1} ref={heading}>{current.title}</h1>
          <p id="onboarding-description" className="onboarding-description">{current.description}</p>
          <div className="onboarding-preview" aria-label="操作示意">
            <span className="onboarding-preview-label">{current.label}<small>操作示意</small></span>
            {current.preview.map((text, index) => (
              <div key={text}><span className="onboarding-preview-dot">{index + 1}</span>{text}</div>
            ))}
          </div>
          <ol className="onboarding-instructions">
            {current.points.map(([title, description]) => (
              <li key={title}><strong>{title}</strong><p>{description}</p></li>
            ))}
          </ol>
        </div>
      </div>
      {error && <p className="onboarding-error" role="alert">{error}</p>}
      <footer className="onboarding-footer">
        <button className="text-button" disabled={busy} onClick={() => { void finish(); }}>跳过教程</button>
        <div>
          {step > 0 && <button className="secondary" disabled={busy} onClick={() => setStep(step - 1)}><ChevronLeft size={15} />上一步</button>}
          {step < steps.length - 1
            ? <button className="primary" disabled={busy} onClick={() => setStep(step + 1)}>下一步<ChevronRight size={15} /></button>
            : <button className="primary" disabled={busy} onClick={() => { void finish(); }}>{busy ? "正在保存…" : "开始使用"}<Check size={15} /></button>}
        </div>
      </footer>
    </dialog>
  );
}
