use crate::{db, models::now};
use anyhow::{anyhow, Result};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use tauri::{Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Presentation {
    #[default]
    Expanded,
    Ball,
    Hidden,
}
#[derive(Default)]
pub struct WindowRuntime(Mutex<Runtime>);
#[derive(Default)]
struct Runtime {
    ready: bool,
    presentation: Presentation,
    widget_size: Option<PhysicalSize<u32>>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
struct Position {
    x: i32,
    y: i32,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
struct Geometry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}
static GEOMETRY: OnceLock<Mutex<HashMap<String, (Instant, Geometry)>>> = OnceLock::new();
fn geometry_queue() -> &'static Mutex<HashMap<String, (Instant, Geometry)>> {
    GEOMETRY.get_or_init(Default::default)
}
fn read_value(c: &Connection, key: &str) -> Result<Option<String>> {
    Ok(
        c.query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
            r.get(0)
        })
        .optional()?,
    )
}
fn write_value(c: &Connection, key: &str, value: &impl Serialize) -> Result<()> {
    c.execute("INSERT INTO settings(key,value,updated_at_utc) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at_utc=excluded.updated_at_utc", rusqlite::params![key, serde_json::to_string(value)?, now()])?;
    Ok(())
}
pub fn saved_presentation(c: &Connection) -> Result<Presentation> {
    read_value(c, "widget_presentation")?
        .map(|v| serde_json::from_str(&v).map_err(Into::into))
        .unwrap_or(Ok(Presentation::Expanded))
}
pub fn validate_saved_state(c: &Connection) -> Result<()> {
    saved_presentation(c)?;
    for label in ["main", "widget"] {
        if let Some(v) = read_value(c, &format!("window_{label}"))? {
            let g: Geometry = serde_json::from_str(&v)?;
            anyhow::ensure!(g.width > 0 && g.height > 0, "窗口尺寸无效");
        }
    }
    if let Some(v) = read_value(c, "window_ball")? {
        serde_json::from_str::<Position>(&v)?;
    }
    Ok(())
}
pub fn save_geometry(window: &tauri::Window) -> Result<()> {
    let Some(runtime) = window.try_state::<WindowRuntime>() else {
        return Ok(());
    };
    if !runtime
        .0
        .lock()
        .map_err(|_| anyhow!("窗口状态锁异常"))?
        .ready
        || !window.is_visible()?
        || window.is_minimized()?
        || window.is_maximized()?
    {
        return Ok(());
    }
    let p = window.outer_position()?;
    let s = window.inner_size()?;
    if window.label() == "widget" {
        runtime
            .0
            .lock()
            .map_err(|_| anyhow!("窗口状态锁异常"))?
            .widget_size = Some(s);
    }
    geometry_queue()
        .lock()
        .map_err(|_| anyhow!("窗口位置锁异常"))?
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
    let mut queue = geometry_queue()
        .lock()
        .map_err(|_| anyhow!("窗口位置锁异常"))?;
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
    let ready: Vec<_> = queue
        .iter()
        .filter(|(_, (at, _))| force || at.elapsed() >= Duration::from_millis(400))
        .map(|(label, _)| label.clone())
        .collect();
    for label in ready {
        if let Some((_, g)) = queue.get(&label) {
            if label == "ball" {
                write_value(&c, "window_ball", &Position { x: g.x, y: g.y })?;
            } else {
                write_value(&c, &format!("window_{label}"), g)?;
            }
        }
        queue.remove(&label);
    }
    Ok(())
}
// Physical coordinates include monitors with negative origins and exclude taskbars.
fn fit(g: Geometry, area: Geometry) -> Geometry {
    let width = g.width.min(area.width);
    let height = g.height.min(area.height);
    Geometry {
        x: (g.x as i64).clamp(area.x as i64, area.x as i64 + (area.width - width) as i64) as i32,
        y: (g.y as i64).clamp(area.y as i64, area.y as i64 + (area.height - height) as i64) as i32,
        width,
        height,
    }
}
fn distance(p: Position, area: Geometry) -> i64 {
    let x = (p.x as i64).clamp(area.x as i64, area.x as i64 + area.width as i64 - 1);
    let y = (p.y as i64).clamp(area.y as i64, area.y as i64 + area.height as i64 - 1);
    (x - p.x as i64)
        .saturating_pow(2)
        .saturating_add((y - p.y as i64).saturating_pow(2))
}
fn place(
    w: &tauri::WebviewWindow,
    position: Position,
    size: PhysicalSize<u32>,
    rescale: bool,
) -> Result<PhysicalSize<u32>> {
    let monitors = w.available_monitors()?;
    let monitor = monitors
        .iter()
        .min_by_key(|m| {
            let a = m.work_area();
            distance(
                position,
                Geometry {
                    x: a.position.x,
                    y: a.position.y,
                    width: a.size.width,
                    height: a.size.height,
                },
            )
        })
        .ok_or_else(|| anyhow!("没有可用显示器"))?;
    let area = monitor.work_area();
    let factor = if rescale {
        monitor.scale_factor() / w.scale_factor()?
    } else {
        1.0
    };
    let (width, height) = if w.label() == "ball" {
        let side = (56.0 * monitor.scale_factor()).round() as u32;
        (side, side)
    } else {
        (
            (size.width as f64 * factor).round() as u32,
            (size.height as f64 * factor).round() as u32,
        )
    };
    let inner = w.inner_size()?;
    let outer = w.outer_size()?;
    let frame_width = outer.width.saturating_sub(inner.width);
    let frame_height = outer.height.saturating_sub(inner.height);
    let g = fit(
        Geometry {
            x: position.x,
            y: position.y,
            width: width.saturating_add(frame_width),
            height: height.saturating_add(frame_height),
        },
        Geometry {
            x: area.position.x,
            y: area.position.y,
            width: area.size.width,
            height: area.size.height,
        },
    );
    w.set_position(PhysicalPosition::new(g.x, g.y))?;
    let size = PhysicalSize::new(
        g.width.saturating_sub(frame_width),
        g.height.saturating_sub(frame_height),
    );
    // A hidden frameless caption is measured differently before its first show.
    if w.label() != "widget" || w.is_visible()? {
        w.set_size(size)?;
    }
    if w.label() == "widget" {
        w.state::<WindowRuntime>()
            .0
            .lock()
            .map_err(|_| anyhow!("窗口状态锁异常"))?
            .widget_size = Some(size);
    }
    Ok(size)
}
fn window(app: &tauri::AppHandle, label: &str) -> Result<tauri::WebviewWindow> {
    app.get_webview_window(label)
        .ok_or_else(|| anyhow!("窗口 {label} 不存在"))
}
pub fn restore_geometry(app: &tauri::AppHandle, label: &str) -> Result<()> {
    let state = app.state::<db::Database>();
    let c = state.lock()?;
    if !db::settings(&c)?.remember_position {
        return Ok(());
    }
    let value = read_value(&c, &format!("window_{label}"))?;
    drop(c);
    if let Some(value) = value {
        let w = window(app, label)?;
        let (p, size) = if label == "ball" {
            (serde_json::from_str::<Position>(&value)?, w.inner_size()?)
        } else {
            let g: Geometry = serde_json::from_str(&value)?;
            (
                Position { x: g.x, y: g.y },
                PhysicalSize::new(g.width, g.height),
            )
        };
        place(&w, p, size, false)?;
    }
    Ok(())
}
fn align(target: &tauri::WebviewWindow, source: &tauri::WebviewWindow) -> Result<()> {
    let p = source.outer_position()?;
    place(
        target,
        Position { x: p.x, y: p.y },
        intended_size(target)?,
        true,
    )?;
    Ok(())
}
fn intended_size(w: &tauri::WebviewWindow) -> Result<PhysicalSize<u32>> {
    if w.label() == "widget" {
        if let Some(size) = w
            .state::<WindowRuntime>()
            .0
            .lock()
            .map_err(|_| anyhow!("窗口状态锁异常"))?
            .widget_size
        {
            return Ok(size);
        }
    }
    Ok(w.inner_size()?)
}
fn show_surface(w: &tauri::WebviewWindow, focus: bool) -> Result<()> {
    let p = w.outer_position()?;
    let size = place(w, Position { x: p.x, y: p.y }, intended_size(w)?, false)?;
    w.show()?;
    // Frameless shadow bounds settle on first show; retain the intended client size.
    w.set_size(size)?;
    place(w, Position { x: p.x, y: p.y }, size, false)?;
    w.unminimize()?;
    if focus {
        w.set_focus()?;
    }
    Ok(())
}
pub fn show_main(app: &tauri::AppHandle, task_id: Option<String>) -> Result<()> {
    let w = window(app, "main")?;
    w.show()?;
    w.unminimize()?;
    w.set_focus()?;
    if let Some(id) = task_id {
        app.emit_to("main", "open-task", id)?;
    }
    Ok(())
}
// Called exclusively on Tauri's main thread. No mutex is held across native calls.
pub fn set_presentation(app: &tauri::AppHandle, next: Presentation) -> Result<()> {
    let runtime = app.state::<WindowRuntime>();
    let previous = runtime
        .0
        .lock()
        .map_err(|_| anyhow!("窗口状态锁异常"))?
        .presentation;
    let widget = window(app, "widget")?;
    let ball = window(app, "ball")?;
    if previous == next {
        if next == Presentation::Expanded {
            show_surface(&widget, true)?;
        }
        if next == Presentation::Ball {
            show_surface(&ball, false)?;
        }
        return Ok(());
    }
    save_geometry(&widget.as_ref().window())?;
    save_geometry(&ball.as_ref().window())?;
    let result = (|| -> Result<()> {
        // Keep the other anchor current even when the next state is hidden.
        if previous == Presentation::Expanded {
            align(&ball, &widget)?;
        }
        if previous == Presentation::Ball {
            align(&widget, &ball)?;
        }
        widget.hide()?;
        ball.hide()?;
        match next {
            Presentation::Expanded => show_surface(&widget, true)?,
            Presentation::Ball => show_surface(&ball, false)?,
            Presentation::Hidden => {}
        }
        let state = app.state::<db::Database>();
        write_value(&*state.lock()?, "widget_presentation", &next)?;
        Ok(())
    })();
    if let Err(error) = result {
        let rollback = (|| -> Result<()> {
            widget.hide()?;
            ball.hide()?;
            match previous {
                Presentation::Expanded => show_surface(&widget, false)?,
                Presentation::Ball => show_surface(&ball, false)?,
                Presentation::Hidden => {}
            }
            Ok(())
        })();
        return match rollback {
            Ok(()) => Err(error),
            Err(e) => Err(anyhow!("{error}；恢复原窗口失败：{e}")),
        };
    }
    runtime
        .0
        .lock()
        .map_err(|_| anyhow!("窗口状态锁异常"))?
        .presentation = next;
    app.emit("data-changed", ())?;
    // Hidden move events are deliberately ignored; save their synchronized anchors here.
    for w in [&widget, &ball] {
        let p = w.outer_position()?;
        let s = intended_size(w)?;
        geometry_queue()
            .lock()
            .map_err(|_| anyhow!("窗口位置锁异常"))?
            .insert(
                w.label().into(),
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
    }
    flush_geometry(app, true)?;
    Ok(())
}
pub fn restore_session(app: &tauri::AppHandle) -> Result<()> {
    let runtime = app.state::<WindowRuntime>();
    runtime
        .0
        .lock()
        .map_err(|_| anyhow!("窗口状态锁异常"))?
        .ready = false;
    let widget = window(app, "widget")?;
    let ball = window(app, "ball")?;
    widget.hide()?;
    ball.hide()?;
    runtime
        .0
        .lock()
        .map_err(|_| anyhow!("窗口状态锁异常"))?
        .widget_size = Some(LogicalSize::new(320.0, 420.0).to_physical(widget.scale_factor()?));
    widget.center()?;
    ball.set_size(LogicalSize::new(56.0, 56.0))?;
    align(&ball, &widget)?;
    restore_geometry(app, "main")?;
    restore_geometry(app, "widget")?;
    restore_geometry(app, "ball")?;
    let state = app.state::<db::Database>();
    let next = saved_presentation(&*state.lock()?)?;
    if !db::onboarding_required(&state.dir)? {
        match next {
            Presentation::Expanded => show_surface(&widget, false)?,
            Presentation::Ball => show_surface(&ball, false)?,
            Presentation::Hidden => {}
        }
    }
    let widget_size = Some(intended_size(&widget)?);
    *runtime.0.lock().map_err(|_| anyhow!("窗口状态锁异常"))? = Runtime {
        ready: true,
        presentation: next,
        widget_size,
    };
    Ok(())
}
pub fn resume_widget_after_onboarding(app: &tauri::AppHandle) -> Result<()> {
    let next = app
        .state::<WindowRuntime>()
        .0
        .lock()
        .map_err(|_| anyhow!("窗口状态锁异常"))?
        .presentation;
    set_presentation(app, next)
}
pub fn restore_database(app: &tauri::AppHandle, path: &Path) -> Result<PathBuf> {
    let runtime = app.state::<WindowRuntime>();
    runtime
        .0
        .lock()
        .map_err(|_| anyhow!("窗口状态锁异常"))?
        .ready = false;
    let result = (|| {
        let mut queue = geometry_queue()
            .lock()
            .map_err(|_| anyhow!("窗口位置锁异常"))?;
        let state = app.state::<db::Database>();
        let backup = state.restore(path)?;
        queue.clear();
        Ok(backup)
    })();
    if result.is_ok() {
        restore_session(app)?;
    }
    runtime
        .0
        .lock()
        .map_err(|_| anyhow!("窗口状态锁异常"))?
        .ready = true;
    result
}
pub fn ball_menu(app: &tauri::AppHandle) -> Result<()> {
    use tauri::menu::{Menu, MenuItem};
    let expand = MenuItem::with_id(app, "widget", "展开小窗口", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide_widget", "隐藏悬浮球", true, None::<&str>)?;
    window(app, "ball")?.popup_menu(&Menu::with_items(app, &[&expand, &hide])?)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fits_work_area_including_taskbar_and_negative_origins() {
        let area = Geometry {
            x: -1920,
            y: -200,
            width: 1920,
            height: 1040,
        };
        assert_eq!(
            fit(
                Geometry {
                    x: -30,
                    y: 820,
                    width: 400,
                    height: 480
                },
                area
            ),
            Geometry {
                x: -400,
                y: 360,
                width: 400,
                height: 480
            }
        );
        assert_eq!(
            fit(
                Geometry {
                    x: -3000,
                    y: -1000,
                    width: 400,
                    height: 480
                },
                area
            )
            .x,
            -1920
        );
        assert_eq!(
            fit(
                Geometry {
                    x: 0,
                    y: 0,
                    width: 3000,
                    height: 2000
                },
                area
            ),
            area
        );
    }
    #[test]
    fn nearest_monitor_uses_physical_coordinates() {
        let left = Geometry {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let right = Geometry {
            x: 0,
            y: 0,
            width: 2560,
            height: 1400,
        };
        assert_eq!(distance(Position { x: -500, y: 50 }, left), 0);
        assert!(
            distance(Position { x: 2500, y: 1600 }, right)
                < distance(Position { x: 2500, y: 1600 }, left)
        );
    }
    #[test]
    fn saved_states_and_ball_position_do_not_change_widget_size() {
        let dir = tempfile::tempdir().unwrap();
        let db = db::Database::open(dir.path()).unwrap();
        {
            let c = db.lock().unwrap();
            assert_eq!(saved_presentation(&c).unwrap(), Presentation::Expanded);
            write_value(
                &c,
                "window_widget",
                &Geometry {
                    x: 120,
                    y: 120,
                    width: 400,
                    height: 480,
                },
            )
            .unwrap();
            write_value(&c, "window_ball", &Position { x: 500, y: 300 }).unwrap();
            for state in [
                Presentation::Expanded,
                Presentation::Ball,
                Presentation::Hidden,
            ] {
                write_value(&c, "widget_presentation", &state).unwrap();
                assert_eq!(saved_presentation(&c).unwrap(), state);
                assert_eq!(
                    db::snapshot(&c, dir.path()).unwrap().widget_presentation,
                    state
                );
                validate_saved_state(&c).unwrap();
            }
        }
        let backup = db.backup("states").unwrap();
        write_value(
            &db.lock().unwrap(),
            "widget_presentation",
            &Presentation::Expanded,
        )
        .unwrap();
        db.restore(&backup).unwrap();
        let c = db.lock().unwrap();
        assert_eq!(saved_presentation(&c).unwrap(), Presentation::Hidden);
        let g: Geometry =
            serde_json::from_str(&read_value(&c, "window_widget").unwrap().unwrap()).unwrap();
        assert_eq!((g.width, g.height), (400, 480));
        write_value(&c, "widget_presentation", &"invalid").unwrap();
        assert!(validate_saved_state(&c).is_err());
    }
}
