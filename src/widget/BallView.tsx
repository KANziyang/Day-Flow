import { useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Leaf } from "lucide-react";
import { useData } from "../hooks/useData";
import { native, windowAction } from "../services/api";
import { movedBeyondClick } from "./ballGesture";

export function BallView() {
  const { run, reportError, error } = useData();
  const gesture = useRef<{ x: number; y: number; dragged: boolean } | null>(null);
  return (
    <button
      className={`floating-ball${error ? " has-error" : ""}`}
      aria-label="展开小窗口"
      title={error || "DayFlow · 点击展开，拖动移动，右键隐藏"}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        gesture.current = { x: e.screenX, y: e.screenY, dragged: false };
        e.currentTarget.setPointerCapture(e.pointerId);
      }}
      onPointerMove={(e) => {
        const current = gesture.current;
        if (!current || current.dragged || !(e.buttons & 1)) return;
        if (movedBeyondClick(current, { x: e.screenX, y: e.screenY })) {
          current.dragged = true;
          if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);
          if (native) void getCurrentWindow().startDragging().catch(reportError);
        }
      }}
      onPointerUp={(e) => {
        const current = gesture.current;
        if (current && movedBeyondClick(current, { x: e.screenX, y: e.screenY })) current.dragged = true;
      }}
      onPointerCancel={() => { if (gesture.current) gesture.current.dragged = true; }}
      onClick={(e) => {
        const dragged = gesture.current?.dragged;
        gesture.current = null;
        if (e.detail === 0 || !dragged) void run(() => windowAction("widget"));
      }}
      onContextMenu={(e) => {
        e.preventDefault();
        gesture.current = null;
        void run(() => windowAction("ball_menu"));
      }}
    >
      <Leaf size={25} aria-hidden="true" />
    </button>
  );
}
