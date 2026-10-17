mod db;
mod models;
mod reports;
mod repository;
#[cfg(test)]
mod tests;
mod windows;
#[cfg(test)]
mod work_record_tests;

use anyhow::{bail, Result};
use db::Database;
use serde_json::{json, Value};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
struct StartupError(String);

fn field<'a>(p: &'a Value, k: &str) -> Result<&'a str> {
    p.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("缺少参数 {k}"))
}

#[tauri::command]
async fn dispatch(
    app: tauri::AppHandle,
    action: String,
    payload: Value,
) -> std::result::Result<Value, String> {
    let result = (|| -> Result<Value> {
        let state = app.try_state::<Database>().ok_or_else(|| {
            anyhow::anyhow!(app
                .try_state::<StartupError>()
                .map(|e| e.0.clone())
                .unwrap_or("本地数据库尚未准备好，请重启应用".into()))
        })?;
        let mut c = state.lock()?;
        let result = match action.as_str() {
            "snapshot" => serde_json::to_value(db::snapshot(&c, &state.dir)?)?,
            "save_task" => serde_json::to_value(repository::save_task(
                &mut c,
                serde_json::from_value(payload)?,
            )?)?,
            "set_status" => serde_json::to_value(repository::set_status(
                &mut c,
                field(&payload, "id")?,
                field(&payload, "status")?,
            )?)?,
            "delete_task" | "restore_task" => {
                repository::delete_task(&mut c, field(&payload, "id")?, action == "restore_task")?;
                Value::Null
            }
            "events" => json!(repository::all_events(&c)?
                .into_iter()
                .filter(|e| payload["id"].as_str().is_none_or(|id| e.task_id == id))
                .collect::<Vec<_>>()),
            "save_log" => {
                serde_json::to_value(repository::save_log(&c, serde_json::from_value(payload)?)?)?
            }
            "delete_log" => {
                c.execute(
                    "DELETE FROM work_logs WHERE id=?1",
                    [field(&payload, "id")?],
                )?;
                Value::Null
            }
            "save_category" => {
                repository::save_category(
                    &c,
                    payload["id"].as_str().map(Into::into),
                    field(&payload, "name")?,
                    field(&payload, "color")?,
                )?;
                Value::Null
            }
            "save_settings" => {
                let mut value = serde_json::to_value(db::settings(&c)?)?;
                let key = field(&payload, "key")?;
                if key == "widget_mode" || value.get(key).is_none() {
                    bail!("设置项无效");
                }
                value[key] = payload
                    .get("value")
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("缺少设置值"))?;
                let s: models::Settings = serde_json::from_value(value)?;
                db::save_settings(&c, &s)?;
                Value::Null
            }
            "report" => serde_json::to_value(reports::generate(
                &c,
                field(&payload, "kind")?,
                field(&payload, "date")?,
            )?)?,
            _ => bail!("未知操作"),
        };
        drop(c);
        if !["snapshot", "events", "report"].contains(&action.as_str()) {
            app.emit("data-changed", ())?;
        }
        Ok(result)
    })();
    result.map_err(|e| e.to_string())
}

fn on_main<T: Send + 'static>(
    app: &tauri::AppHandle,
    f: impl FnOnce(tauri::AppHandle) -> Result<T> + Send + 'static,
) -> Result<T> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(f(handle));
    })?;
    rx.recv().map_err(|_| anyhow::anyhow!("窗口操作未完成"))?
}
#[tauri::command]
async fn window_action(
    app: tauri::AppHandle,
    action: String,
    task_id: Option<String>,
) -> std::result::Result<Value, String> {
    let result = (|| -> Result<Value> {
        match action.as_str() {
            "main" => {
                on_main(&app, move |a| windows::show_main(&a, task_id))?;
                Ok(Value::Null)
            }
            "widget" | "hide_widget" => {
                on_main(&app, move |a| {
                    windows::set_widget_visible(&a, action == "widget")
                })?;
                Ok(Value::Null)
            }
            "desktop" | "topmost" => {
                let mode = action;
                let result = on_main(&app, move |a| {
                    let status = windows::apply_mode(&a, &mode)?;
                    let state = a.state::<Database>();
                    let c = state.lock()?;
                    let mut s = db::settings(&c)?;
                    s.widget_mode = mode;
                    db::save_settings(&c, &s)?;
                    a.emit("data-changed", ())?;
                    Ok(status)
                })?;
                Ok(serde_json::to_value(result)?)
            }
            "mode_status" => {
                let s = app
                    .state::<RuntimeStatus>()
                    .0
                    .lock()
                    .map_err(|_| anyhow::anyhow!("窗口状态异常"))?
                    .clone();
                Ok(serde_json::to_value(s)?)
            }
            "quit" => {
                app.exit(0);
                Ok(Value::Null)
            }
            _ => bail!("未知窗口操作"),
        }
    })();
    result.map_err(|e| e.to_string())
}
pub struct RuntimeStatus(pub std::sync::Mutex<windows::desktop_host::ModeStatus>);

#[tauri::command]
async fn data_action(app: tauri::AppHandle, action: String) -> std::result::Result<String, String> {
    let result = (|| -> Result<String> {
        let state = app.state::<Database>();
        match action.as_str() {
            "backup" => Ok(state.backup("manual")?.to_string_lossy().into()),
            "export" => Ok(state.export()?.to_string_lossy().into()),
            "open_dir" => {
                std::process::Command::new("explorer.exe")
                    .arg(&state.dir)
                    .spawn()?;
                Ok(state.dir.to_string_lossy().into())
            }
            "restore" => {
                let file = app
                    .dialog()
                    .file()
                    .set_title("选择 DayFlow 数据库备份")
                    .add_filter("DayFlow 数据库", &["db"])
                    .blocking_pick_file();
                let Some(file) = file else {
                    return Ok("已取消恢复".into());
                };
                let path = file.into_path().map_err(|e| anyhow::anyhow!(e))?;
                let confirmed=app.dialog().message("恢复将替换当前任务、日志与设置。应用会先保存现有数据库的完整安全备份。是否继续？").title("确认恢复数据库").kind(tauri_plugin_dialog::MessageDialogKind::Warning).buttons(tauri_plugin_dialog::MessageDialogButtons::YesNo).blocking_show();
                if !confirmed {
                    return Ok("已取消恢复".into());
                }
                let safety = state.restore(&path)?;
                app.emit("data-changed", ())?;
                let settings = db::settings(&*state.lock()?)?;
                on_main(&app, move |a| {
                    windows::apply_mode(&a, &settings.widget_mode)
                })?;
                Ok(format!("恢复完成；原数据备份：{}", safety.display()))
            }
            _ => bail!("未知数据操作"),
        }
    })();
    result.map_err(|e| e.to_string())
}
#[tauri::command]
async fn autostart(
    app: tauri::AppHandle,
    enabled: Option<bool>,
) -> std::result::Result<bool, String> {
    let manager = app.autolaunch();
    if let Some(enabled) = enabled {
        if enabled {
            manager.enable()
        } else {
            manager.disable()
        }
        .map_err(|e| e.to_string())?;
    }
    manager.is_enabled().map_err(|e| e.to_string())
}

fn tray(app: &tauri::App) -> Result<()> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::TrayIconBuilder,
    };
    let main = MenuItem::with_id(app, "main", "打开 DayFlow", true, None::<&str>)?;
    let widget = MenuItem::with_id(app, "widget", "显示小组件", true, None::<&str>)?;
    let mode = MenuItem::with_id(app, "mode", "切换桌面 / 置顶", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "exit", "退出 DayFlow", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&main, &widget, &mode, &exit])?;
    TrayIconBuilder::new()
        .icon(
            app.default_window_icon()
                .ok_or_else(|| anyhow::anyhow!("应用图标缺失"))?
                .clone(),
        )
        .tooltip("DayFlow · 让每一天有迹可循")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let result = (|| -> Result<()> {
                match event.id().as_ref() {
                    "main" => windows::show_main(app, None)?,
                    "widget" => windows::set_widget_visible(app, true)?,
                    "mode" => {
                        let state = app.state::<Database>();
                        let mut s = {
                            let c = state.lock()?;
                            db::settings(&c)?
                        };
                        s.widget_mode = if s.widget_mode == "topmost" {
                            "desktop"
                        } else {
                            "topmost"
                        }
                        .into();
                        windows::apply_mode(app, &s.widget_mode)?;
                        db::save_settings(&*state.lock()?, &s)?;
                        app.emit("data-changed", ())?;
                    }
                    "exit" => app.exit(0),
                    _ => {}
                }
                Ok(())
            })();
            if let Err(e) = result {
                let _ = app.emit("app-error", e.to_string());
            }
        })
        .build(app)?;
    Ok(())
}

fn background(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut last_check = std::time::Instant::now();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            if let Err(e) = windows::flush_geometry(&app, false) {
                let _ = app.emit("app-error", e.to_string());
            }
            if last_check.elapsed() < std::time::Duration::from_secs(30) {
                continue;
            }
            last_check = std::time::Instant::now();
            if windows::desktop_host::host_lost() {
                let a = app.clone();
                let _ = app.run_on_main_thread(move || {
                    let _ = windows::apply_mode(&a, "desktop");
                });
            }
            let result = (|| -> Result<()> {
                use tauri_plugin_notification::NotificationExt;
                let state = app.state::<Database>();
                let c = state.lock()?;
                let s = db::settings(&c)?;
                if !s.notifications {
                    return Ok(());
                }
                let local = chrono::Local::now();
                let time = local.format("%H:%M").to_string();
                let quiet = if s.quiet_start == s.quiet_end {
                    false
                } else if s.quiet_start < s.quiet_end {
                    time >= s.quiet_start && time < s.quiet_end
                } else {
                    time >= s.quiet_start || time < s.quiet_end
                };
                if quiet {
                    return Ok(());
                }
                for t in repository::tasks(&c)?.into_iter().filter(|t| {
                    t.deleted_at_utc.is_none()
                        && ["todo", "in_progress"].contains(&t.status.as_str())
                }) {
                    let Some(d) = &t.due_date_local else {
                        continue;
                    };
                    let due = if let Some(at) = &t.due_at_utc {
                        chrono::DateTime::parse_from_rfc3339(at)?.timestamp() <= local.timestamp()
                    } else {
                        d < &local.date_naive().to_string()
                            || (d == &local.date_naive().to_string()
                                && time >= s.date_reminder_time)
                    };
                    if !due {
                        continue;
                    }
                    let key = format!("{}T{}", d, t.due_time_local.as_deref().unwrap_or("date"));
                    let inserted=c.execute("INSERT OR IGNORE INTO notification_deliveries(task_id,due_key,sent_at_utc) VALUES(?1,?2,?3)",rusqlite::params![t.id,key,models::now()])?;
                    if inserted > 0 {
                        if let Err(e) = app
                            .notification()
                            .builder()
                            .title("DayFlow · 任务到期提醒")
                            .body(&t.title)
                            .show()
                        {
                            c.execute("DELETE FROM notification_deliveries WHERE task_id=?1 AND due_key=?2",rusqlite::params![t.id,key])?;
                            return Err(e.into());
                        }
                    }
                }
                Ok(())
            })();
            if let Err(e) = result {
                let _ = app.emit("app-error", e.to_string());
            }
        }
    });
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = windows::show_main(app, None);
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--autostart"])
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(RuntimeStatus(std::sync::Mutex::new(
            windows::desktop_host::ModeStatus {
                mode: "desktop".into(),
                embedded: false,
                message: "桌面兼容模式 · 初始化中".into(),
            },
        )))
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            #[cfg(debug_assertions)]
            let dir = std::env::var_os("DAYFLOW_TEST_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(dir);
            let database = match Database::open(&dir) {
                Ok(database)=>database,
                Err(error)=>{
                    app.manage(StartupError(format!("无法打开本地数据库：{error}。数据目录：{}。请修复文件访问或数据库问题后重启 DayFlow。",dir.display())));
                    if let Some(w)=app.get_webview_window("main"){w.show()?;}
                    return Ok(());
                }
            };
            let daily_prefix = format!("auto-{}", chrono::Local::now().format("%Y%m%d"));
            let exists = std::fs::read_dir(dir.join("backups"))
                .ok()
                .is_some_and(|entries| {
                    entries
                        .flatten()
                        .any(|e| e.file_name().to_string_lossy().starts_with(&daily_prefix))
                });
            if !exists {
                database.backup(&daily_prefix)?;
            }
            app.manage(database);
            tray(app)?;
            windows::restore_geometry(app.handle(), "main")?;
            windows::restore_geometry(app.handle(), "widget")?;
            let s = {
                let state = app.state::<Database>();
                let c = state.lock()?;
                db::settings(&c)?
            };
            windows::apply_mode(app.handle(), &s.widget_mode)?;
            windows::set_widget_visible(app.handle(), true)?;
            if !std::env::args().any(|a| a == "--autostart") {
                windows::show_main(app.handle(), None)?;
            }
            background(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                if window.try_state::<Database>().is_none(){window.app_handle().exit(0);return;}
                if window.label() == "main" {
                    let state = window.state::<Database>();
                    let close = state
                        .lock()
                        .and_then(|c| db::settings(&c))
                        .map(|s| s.close_to_tray)
                        .unwrap_or(true);
                    api.prevent_close();
                    if close {
                        let _ = window.hide();
                    } else {
                        let _ = window.emit("main-close-request", ());
                    }
                } else {
                    api.prevent_close();
                    if let Err(e) = windows::set_widget_visible(window.app_handle(), false) {
                        let _ = window.emit("app-error", e.to_string());
                    }
                }
            }
            tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                if let Err(e) = windows::save_geometry(window) {
                    let _ = window.emit("app-error", e.to_string());
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            dispatch,
            window_action,
            data_action,
            autostart
        ])
        .build(tauri::generate_context!())
        .expect("DayFlow 启动失败")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
                // Release Explorer parenting while the event loop and HWND are
                // still alive, before Tauri's cleanup hides top-level windows.
                let _ = windows::flush_geometry(app, true);
                if let Some(w) = app.get_webview_window("widget") {
                    if let Err(e) = windows::set_widget_visible(app, false) {
                        eprintln!("退出时隐藏小组件失败：{e}");
                    }
                    if let Err(e) = windows::desktop_host::detach(&w) {
                        eprintln!("退出时解除桌面挂载失败：{e}");
                    }
                }
            }
            if matches!(event, tauri::RunEvent::Exit) {
                let _ = windows::flush_geometry(app, true);
            }
        });
}
