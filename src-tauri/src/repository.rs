use crate::models::*;
use anyhow::{bail, Result};
use chrono::{Local, NaiveDate, NaiveTime, TimeZone};
use rusqlite::{params, Connection, OptionalExtension, Row};

pub fn validate_date(s: &str) -> Result<NaiveDate> {
    let d =
        NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| anyhow::anyhow!("请输入有效日期"))?;
    if d.format("%Y-%m-%d").to_string() != s || !("1900-01-01"..="9999-12-31").contains(&s) {
        bail!("日期须为 1900 至 9999 年的 YYYY-MM-DD");
    }
    Ok(d)
}
pub fn validate_time(s: &str) -> Result<NaiveTime> {
    let t = NaiveTime::parse_from_str(s, "%H:%M")
        .map_err(|_| anyhow::anyhow!("请输入有效时间 HH:mm"))?;
    if t.format("%H:%M").to_string() != s {
        bail!("时间格式须为 HH:mm");
    }
    Ok(t)
}
fn task_row(r: &Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: r.get(0)?,
        title: r.get(1)?,
        description: r.get(2)?,
        status: r.get(3)?,
        priority: r.get(4)?,
        category_id: r.get(5)?,
        due_date_local: r.get(6)?,
        due_time_local: r.get(7)?,
        due_at_utc: r.get(8)?,
        created_at_utc: r.get(9)?,
        updated_at_utc: r.get(10)?,
        completed_at_utc: r.get(11)?,
        deleted_at_utc: r.get(12)?,
    })
}
const TASK_COLUMNS:&str="id,title,description,status,priority,category_id,due_date_local,due_time_local,due_at_utc,created_at_utc,updated_at_utc,completed_at_utc,deleted_at_utc";
pub fn tasks(c: &Connection) -> Result<Vec<Task>> {
    Ok(c.prepare(&format!(
        "SELECT {TASK_COLUMNS} FROM tasks ORDER BY created_at_utc"
    ))?
    .query_map([], task_row)?
    .collect::<rusqlite::Result<_>>()?)
}
pub fn task(c: &Connection, id: &str) -> Result<Task> {
    Ok(c.query_row(
        &format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id=?1"),
        [id],
        task_row,
    )
    .optional()?
    .ok_or_else(|| anyhow::anyhow!("任务不存在"))?)
}
pub fn categories(c: &Connection) -> Result<Vec<Category>> {
    Ok(
        c.prepare(
            "SELECT id,name,color,created_at_utc FROM categories ORDER BY created_at_utc,id",
        )?
        .query_map([], |r| {
            Ok(Category {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                created_at_utc: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?,
    )
}
pub fn logs(c: &Connection) -> Result<Vec<WorkLog>> {
    Ok(c.prepare("SELECT id,task_id,local_date,content,created_at_utc,updated_at_utc FROM work_logs ORDER BY local_date DESC,created_at_utc DESC")?.query_map([],|r|Ok(WorkLog{id:r.get(0)?,task_id:r.get(1)?,local_date:r.get(2)?,content:r.get(3)?,created_at_utc:r.get(4)?,updated_at_utc:r.get(5)?}))?.collect::<rusqlite::Result<_>>()?)
}
pub fn all_events(c: &Connection) -> Result<Vec<TaskEvent>> {
    Ok(c.prepare("SELECT id,task_id,event_type,old_value,new_value,task_title_snapshot,snapshot_json,created_at_utc FROM task_events ORDER BY created_at_utc,rowid")?.query_map([],|r|Ok(TaskEvent{id:r.get(0)?,task_id:r.get(1)?,event_type:r.get(2)?,old_value:r.get(3)?,new_value:r.get(4)?,task_title_snapshot:r.get(5)?,snapshot_json:r.get(6)?,created_at_utc:r.get(7)?}))?.collect::<rusqlite::Result<_>>()?)
}
pub fn event(
    c: &Connection,
    t: &Task,
    kind: &str,
    old: Option<&str>,
    new: Option<&str>,
    at: &str,
) -> Result<()> {
    let mut snapshot = serde_json::to_value(t)?;
    let name: Option<String> = match &t.category_id {
        Some(id) => c
            .query_row("SELECT name FROM categories WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?,
        None => None,
    };
    snapshot["category_name_snapshot"] = serde_json::to_value(name)?;
    c.execute("INSERT INTO task_events(id,task_id,event_type,old_value,new_value,task_title_snapshot,snapshot_json,created_at_utc) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![id(),t.id,kind,old,new,t.title,serde_json::to_string(&snapshot)?,at])?;
    Ok(())
}
fn write_task(c: &Connection, t: &Task) -> Result<()> {
    c.execute("INSERT INTO tasks(id,title,description,status,priority,category_id,due_date_local,due_time_local,due_at_utc,created_at_utc,updated_at_utc,completed_at_utc,deleted_at_utc) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13) ON CONFLICT(id) DO UPDATE SET title=excluded.title,description=excluded.description,status=excluded.status,priority=excluded.priority,category_id=excluded.category_id,due_date_local=excluded.due_date_local,due_time_local=excluded.due_time_local,due_at_utc=excluded.due_at_utc,updated_at_utc=excluded.updated_at_utc,completed_at_utc=excluded.completed_at_utc,deleted_at_utc=excluded.deleted_at_utc",params![t.id,t.title,t.description,t.status,t.priority,t.category_id,t.due_date_local,t.due_time_local,t.due_at_utc,t.created_at_utc,t.updated_at_utc,t.completed_at_utc,t.deleted_at_utc])?;
    Ok(())
}
fn change_status(t: &mut Task, status: &str, at: &str) -> Result<()> {
    if !["todo", "in_progress", "completed", "cancelled"].contains(&status) {
        bail!("任务状态无效");
    }
    if t.status != status {
        t.completed_at_utc = if status == "completed" {
            Some(at.into())
        } else {
            None
        };
        t.status = status.into();
    }
    Ok(())
}
pub fn save_task(c: &mut Connection, input: TaskInput) -> Result<Task> {
    let title = input.title.trim();
    if title.is_empty() || title.chars().count() > 500 {
        bail!("任务标题须为 1 至 500 字");
    }
    if input.description.chars().count() > 20000 {
        bail!("描述最多 20000 字");
    }
    if !["high", "medium", "low"].contains(&input.priority.as_str()) {
        bail!("重要程度无效");
    }
    let date = input
        .due_date_local
        .as_deref()
        .map(validate_date)
        .transpose()?;
    let time = input
        .due_time_local
        .as_deref()
        .map(validate_time)
        .transpose()?;
    if time.is_some() && date.is_none() {
        bail!("设置截止时刻前请选择日期");
    }
    let tx = c.transaction()?;
    let at = now();
    let old = input.id.as_deref().map(|id| task(&tx, id)).transpose()?;
    if let Some(t) = &old {
        if t.deleted_at_utc.is_some() {
            bail!("请先恢复已删除任务");
        }
        if input.expected_updated_at.as_deref() != Some(&t.updated_at_utc) {
            bail!("任务已在另一个窗口修改，请重新载入后再保存");
        }
    }
    let due_unchanged = old.as_ref().is_some_and(|old| {
        old.due_date_local == input.due_date_local && old.due_time_local == input.due_time_local
    });
    let due_at = if due_unchanged {
        old.as_ref().and_then(|old| old.due_at_utc.clone())
    } else {
        match (date, time) {
            (Some(d), Some(t)) => Some(
                Local
                    .from_local_datetime(&d.and_time(t))
                    .single()
                    .ok_or_else(|| {
                        anyhow::anyhow!("该本地时刻因夏令时不存在或存在歧义，请选择其他时刻")
                    })?
                    .with_timezone(&chrono::Utc)
                    .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
            ),
            _ => None,
        }
    };
    let mut t = old.clone().unwrap_or_else(|| Task {
        id: id(),
        title: String::new(),
        description: String::new(),
        status: "todo".into(),
        priority: input.priority.clone(),
        category_id: None,
        due_date_local: None,
        due_time_local: None,
        due_at_utc: None,
        created_at_utc: at.clone(),
        updated_at_utc: at.clone(),
        completed_at_utc: None,
        deleted_at_utc: None,
    });
    t.title = title.into();
    t.description = input.description;
    t.priority = input.priority;
    t.category_id = input.category_id;
    t.due_date_local = input.due_date_local;
    t.due_time_local = input.due_time_local;
    t.due_at_utc = due_at;
    change_status(&mut t, &input.status, &at)?;
    if old.as_ref() == Some(&t) {
        return Ok(t);
    }
    t.updated_at_utc = at.clone();
    write_task(&tx, &t)?;
    if let Some(o) = old {
        let mut specific = false;
        if o.status != t.status {
            status_event(&tx, &o, &t, &at)?;
            specific = true;
        }
        if o.due_date_local != t.due_date_local || o.due_time_local != t.due_time_local {
            event(
                &tx,
                &t,
                "due_changed",
                Some(&format!(
                    "{} {}",
                    o.due_date_local.as_deref().unwrap_or(""),
                    o.due_time_local.as_deref().unwrap_or("")
                )),
                Some(&format!(
                    "{} {}",
                    t.due_date_local.as_deref().unwrap_or(""),
                    t.due_time_local.as_deref().unwrap_or("")
                )),
                &at,
            )?;
            specific = true;
        }
        if o.priority != t.priority {
            event(
                &tx,
                &t,
                "priority_changed",
                Some(&o.priority),
                Some(&t.priority),
                &at,
            )?;
            specific = true;
        }
        if !specific
            || o.title != t.title
            || o.description != t.description
            || o.category_id != t.category_id
        {
            event(&tx, &t, "updated", None, None, &at)?;
        }
    } else {
        event(&tx, &t, "created", None, None, &at)?;
        if t.status == "completed" {
            event(&tx, &t, "completed", Some("todo"), Some("completed"), &at)?;
        }
    }
    tx.commit()?;
    Ok(t)
}
fn status_event(c: &Connection, o: &Task, t: &Task, at: &str) -> Result<()> {
    event(
        c,
        t,
        if t.status == "completed" {
            "completed"
        } else if o.status == "completed" {
            "reopened"
        } else {
            "status_changed"
        },
        Some(&o.status),
        Some(&t.status),
        at,
    )
}
pub fn set_status(c: &mut Connection, id: &str, status: &str) -> Result<Task> {
    let tx = c.transaction()?;
    let o = task(&tx, id)?;
    if o.deleted_at_utc.is_some() {
        bail!("任务已删除");
    }
    if o.status == status {
        return Ok(o);
    }
    let at = now();
    let mut t = o.clone();
    change_status(&mut t, status, &at)?;
    t.updated_at_utc = at.clone();
    write_task(&tx, &t)?;
    status_event(&tx, &o, &t, &at)?;
    tx.commit()?;
    Ok(t)
}
pub fn delete_task(c: &mut Connection, id: &str, restore: bool) -> Result<()> {
    let tx = c.transaction()?;
    let mut t = task(&tx, id)?;
    if t.deleted_at_utc.is_none() == restore {
        return Ok(());
    }
    let at = now();
    t.deleted_at_utc = if restore { None } else { Some(at.clone()) };
    t.updated_at_utc = at.clone();
    write_task(&tx, &t)?;
    event(
        &tx,
        &t,
        if restore { "restored" } else { "deleted" },
        None,
        None,
        &at,
    )?;
    tx.commit()?;
    Ok(())
}
pub fn save_log(c: &Connection, i: LogInput) -> Result<WorkLog> {
    validate_date(&i.local_date)?;
    let content = i.content.trim();
    if content.is_empty() || content.chars().count() > 20000 {
        bail!("工作内容须为 1 至 20000 字");
    }
    if let Some(task_id) = &i.task_id {
        task(c, task_id)?;
    }
    let at = now();
    let mut l = WorkLog {
        id: i.id.clone().unwrap_or_else(id),
        task_id: i.task_id,
        local_date: i.local_date,
        content: content.into(),
        created_at_utc: at.clone(),
        updated_at_utc: at,
    };
    if i.id.is_some() {
        let old: Option<(String, String)> = c
            .query_row(
                "SELECT created_at_utc,updated_at_utc FROM work_logs WHERE id=?1",
                [&l.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (created, updated) = old.ok_or_else(|| anyhow::anyhow!("日志不存在"))?;
        if i.expected_updated_at.as_deref() != Some(&updated) {
            bail!("日志已在另一个窗口修改，请重新载入");
        }
        l.created_at_utc = created;
    }
    c.execute("INSERT INTO work_logs(id,task_id,local_date,content,created_at_utc,updated_at_utc) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET task_id=excluded.task_id,local_date=excluded.local_date,content=excluded.content,updated_at_utc=excluded.updated_at_utc",params![l.id,l.task_id,l.local_date,l.content,l.created_at_utc,l.updated_at_utc])?;
    Ok(l)
}
pub fn save_category(c: &Connection, id_: Option<String>, name: &str, color: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 30 {
        bail!("分类名须为 1 至 30 字");
    }
    if color.len() != 7
        || !color.starts_with('#')
        || !color[1..].chars().all(|x| x.is_ascii_hexdigit())
    {
        bail!("分类颜色无效");
    }
    if let Some(id) = id_ {
        if c.execute(
            "UPDATE categories SET name=?1,color=?2 WHERE id=?3",
            params![name, color, id],
        )? == 0
        {
            bail!("分类不存在");
        }
    } else {
        c.execute(
            "INSERT INTO categories(id,name,color,created_at_utc) VALUES(?1,?2,?3,?4)",
            params![id(), name, color, now()],
        )?;
    }
    Ok(())
}
