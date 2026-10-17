export type Status = "todo" | "in_progress" | "completed" | "cancelled";
export type Priority = "high" | "medium" | "low";
export interface Task {
  id: string;
  title: string;
  description: string;
  status: Status;
  priority: Priority;
  category_id: string | null;
  due_date_local: string | null;
  due_time_local: string | null;
  due_at_utc: string | null;
  created_at_utc: string;
  updated_at_utc: string;
  completed_at_utc: string | null;
  deleted_at_utc: string | null;
}
export interface TaskInput {
  id?: string;
  title: string;
  description: string;
  status: Status;
  priority: Priority;
  category_id: string | null;
  due_date_local: string | null;
  due_time_local: string | null;
  expected_updated_at?: string;
}
export interface Category {
  id: string;
  name: string;
  color: string;
  created_at_utc: string;
}
export interface WorkLog {
  id: string;
  task_id: string | null;
  local_date: string;
  content: string;
  created_at_utc: string;
  updated_at_utc: string;
}
export interface TaskEvent {
  id: string;
  task_id: string;
  event_type: string;
  old_value: string | null;
  new_value: string | null;
  task_title_snapshot: string;
  snapshot_json: string;
  created_at_utc: string;
}
export interface Settings {
  widget_mode: "desktop" | "topmost";
  widget_count: 3 | 5 | 8;
  widget_opacity: number;
  remember_position: boolean;
  show_completed: boolean;
  theme: "light" | "dark" | "system";
  close_to_tray: boolean;
  completion_animation: boolean;
  default_priority: Priority;
  notifications: boolean;
  quiet_start: string;
  quiet_end: string;
  date_reminder_time: string;
}
export interface Snapshot {
  tasks: Task[];
  categories: Category[];
  logs: WorkLog[];
  settings: Settings;
  data_dir: string;
  completion_events: [string, string][];
}
export interface ModeStatus {
  mode: string;
  embedded: boolean;
  message: string;
}
export interface Report {
  kind: string;
  start: string;
  end: string;
  created: number;
  completed: number;
  completion_events: number;
  in_progress: number;
  completions: TaskEvent[];
  logs: WorkLog[];
  unfinished: Task[];
  upcoming: Task[];
  daily: [string, number][];
  weekly: [string, number][];
  categories: [string, number][];
}
export const statusLabels: Record<Status, string> = {
  todo: "待办",
  in_progress: "进行中",
  completed: "已完成",
  cancelled: "已取消",
};
export const priorityLabels: Record<Priority, string> = {
  high: "高",
  medium: "中",
  low: "低",
};
