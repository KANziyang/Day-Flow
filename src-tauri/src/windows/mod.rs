use crate::{db, models::*};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize};

#[derive(Serialize, Deserialize)]
struct Geometry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}
static GEOMETRY: OnceLock<Mutex<HashMap<String, (Instant, Geometry)>>> = OnceLock::new();
pub fn save_geometry(window: &tauri::Window) -> Result<()> {
    if window.is_minimized()? || window.is_maximized()? {
        return Ok(());
    }
    let p = window.outer_position()?;
    let s = window.inner_size()?;
    GEOMETRY
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| anyhow::anyhow!("窗口位置锁异常"))?
        .insert(
            window.label().into(),
            (
                Instant::now(),
                Geometry {
                    x: p.x,
                    y: p.y,
                    width: s.width,
                    height: s.height,
                },
            ),
        );
    Ok(())
}
pub fn flush_geometry(app: &tauri::AppHandle, force: bool) -> Result<()> {
    let Some(queue) = GEOMETRY.get() else {
        return Ok(());
    };
    let mut queue = queue
        .lock()
        .map_err(|_| anyhow::anyhow!("窗口位置锁异常"))?;
    if queue.is_empty() {
        return Ok(());
    }
    let Some(state) = app.try_state::<db::Database>() else {
        return Ok(());
    };
    let c = state.lock()?;
    if !db::settings(&c)?.remember_position {
        queue.clear();
        return Ok(());
    }
    let ready: Vec<String> = queue
        .iter()
        .filter(|(_, (at, _))| force || at.elapsed() >= Duration::from_millis(400))
        .map(|(label, _)| label.clone())
        .collect();
    for label in ready {
        if let Some((_, g)) = queue.get(&label) {
            c.execute("INSERT INTO settings(key,value,updated_at_utc) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at_utc=excluded.updated_at_utc",rusqlite::params![format!("window_{label}"),serde_json::to_string(g)?,now()])?;
        }
        queue.remove(&label);
    }
    Ok(())
}
pub fn restore_geometry(app: &tauri::AppHandle, label: &str) -> Result<()> {
    let state = app.state::<db::Database>();
    let c = state.lock()?;
    if !db::settings(&c)?.remember_position {
        return Ok(());
    }
    use rusqlite::OptionalExtension;
    let value: Option<String> = c
        .query_row(
            "SELECT value FROM settings WHERE key=?1",
            [format!("window_{label}")],
            |r| r.get(0),
        )
        .optional()?;
    drop(c);
    if let (Some(value), Some(w)) = (value, app.get_webview_window(label)) {
        let g: Geometry = serde_json::from_str(&value)?;
        let monitors = w.available_monitors()?;
        let usable = monitors.iter().find(|m| {
            let p = m.position();
            let s = m.size();
            g.x < p.x + s.width as i32 - 80
                && g.x + g.width as i32 > p.x + 80
                && g.y >= p.y
                && g.y < p.y + s.height as i32 - 60
        });
        if let Some(m) = usable {
            let s = m.size();
            w.set_size(PhysicalSize::new(
                g.width.min(s.width).max(260),
                g.height.min(s.height).max(300),
            ))?;
            let p = m.position();
            w.set_position(PhysicalPosition::new(
                g.x.max(p.x).min(p.x + s.width as i32 - 200),
                g.y.max(p.y).min(p.y + s.height as i32 - 100),
            ))?;
        } else {
            w.center()?;
        }
    }
    Ok(())
}
pub fn show_main(app: &tauri::AppHandle, task_id: Option<String>) -> Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        w.show()?;
        w.unminimize()?;
        w.set_focus()?;
        if let Some(id) = task_id {
            app.emit_to("main", "open-task", id)?;
        }
    }
    Ok(())
}
pub fn set_widget_visible(app: &tauri::AppHandle, visible: bool) -> Result<()> {
    let w = app
        .get_webview_window("widget")
        .ok_or_else(|| anyhow::anyhow!("小组件窗口不存在"))?;
    if visible {
        w.show()?;
    } else {
        w.hide()?;
    }
    Ok(())
}
