import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  Report,
  Snapshot,
  Task,
  TaskEvent,
  TaskInput,
  Status,
  WorkLog,
} from "../types";

export const native = isTauri();
export function request<T>(action: string, payload: unknown = {}): Promise<T> {
  if (!native)
    return Promise.reject(
      new Error("请通过 Tauri 启动 DayFlow。浏览器预览无法访问本地数据库。"),
    );
  return invoke<T>("dispatch", { action, payload });
}
export const api = {
  snapshot: () => request<Snapshot>("snapshot"),
  finishOnboarding: () => request<void>("finish_onboarding"),
  saveTask: (input: TaskInput) => request<Task>("save_task", input),
  status: (id: string, status: Status) =>
    request<Task>("set_status", { id, status }),
  events: (id: string) => request<TaskEvent[]>("events", { id }),
  report: (kind: string, date: string) =>
    request<Report>("report", { kind, date }),
  saveLog: (
    input: Omit<WorkLog, "id" | "created_at_utc" | "updated_at_utc"> & {
      id?: string;
      expected_updated_at?: string;
    },
  ) => request<WorkLog>("save_log", input),
};
export function windowAction(action: "main" | "widget" | "collapse_widget" | "show_ball" | "hide_widget" | "ball_menu" | "quit", taskId?: string) {
  return invoke<void>("window_action", {
    action,
    taskId: taskId ?? null,
  });
}
export function dataAction(action: string) {
  return invoke<string>("data_action", { action });
}
export function autostart(enabled?: boolean) {
  return invoke<boolean>("autostart", { enabled: enabled ?? null });
}
export function errorText(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
