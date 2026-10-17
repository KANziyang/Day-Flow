use crate::models::*;
use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection};
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
    time::Duration,
};

pub struct Database {
    pub conn: Mutex<Connection>,
    pub dir: PathBuf,
}
impl Database {
    pub fn open(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let mut conn = Connection::open(dir.join("dayflow.db"))?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        Self::migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            dir: dir.into(),
        })
    }
    pub fn lock(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁异常，请重启应用"))
    }
    pub fn migrate(conn: &mut Connection) -> Result<()> {
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 3 {
            bail!("数据库由更高版本创建，不能用旧版打开");
        }
        if version < 1 {
            let tx = conn.transaction()?;
            tx.execute_batch(include_str!("../migrations/0001_init.sql"))?;
            tx.pragma_update(None, "user_version", 1)?;
            tx.commit()?;
        }
        if version < 2 {
            let tx = conn.transaction()?;
            tx.execute_batch(include_str!("../migrations/0002_work_records.sql"))?;
            tx.pragma_update(None, "user_version", 2)?;
            tx.commit()?;
        }
        if version < 3 {
            let tx = conn.transaction()?;
            tx.execute_batch(include_str!("../migrations/0003_remove_widget_mode.sql"))?;
            tx.pragma_update(None, "user_version", 3)?;
            tx.commit()?;
        }
        Ok(())
    }
    pub fn backup(&self, prefix: &str) -> Result<PathBuf> {
        let conn = self.lock()?;
        Self::backup_conn(&conn, &self.dir, prefix)
    }
    pub fn backup_conn(conn: &Connection, dir: &Path, prefix: &str) -> Result<PathBuf> {
        let folder = dir.join("backups");
        std::fs::create_dir_all(&folder)?;
        let path = folder.join(format!(
            "{prefix}-{}-{}.db",
            chrono::Local::now().format("%Y%m%d-%H%M%S"),
            &id()[..8]
        ));
        let mut dest = Connection::open(&path)?;
        rusqlite::backup::Backup::new(conn, &mut dest)?.run_to_completion(
            128,
            Duration::from_millis(5),
            None,
        )?;
        Ok(path)
    }
    pub fn restore(&self, path: &Path) -> Result<PathBuf> {
        if path.canonicalize()? == self.dir.join("dayflow.db").canonicalize()? {
            bail!("不能用正在使用的数据库恢复自身");
        }
        let source = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let version: i64 = source.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if ![1, 2, 3].contains(&version) {
            bail!("不是受支持的 DayFlow 数据库备份");
        }
        let mut staging = Connection::open_in_memory()?;
        rusqlite::backup::Backup::new(&source, &mut staging)?.run_to_completion(
            128,
            Duration::from_millis(5),
            None,
        )?;
        staging.execute_batch("PRAGMA foreign_keys=ON;")?;
        Self::migrate(&mut staging).context("备份数据升级失败")?;
        validate_backup(&staging, &self.dir)?;
        let mut conn = self.lock()?;
        let safety = Self::backup_conn(&conn, &self.dir, "before-restore")?;
        rusqlite::backup::Backup::new(&staging, &mut conn)?
            .run_to_completion(128, Duration::from_millis(5), None)
            .context("恢复失败，原数据库已保留安全备份")?;
        conn.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        Ok(safety)
    }
    pub fn export(&self) -> Result<PathBuf> {
        let conn = self.lock()?;
        let value = serde_json::json!({"format":"DayFlow", "version":3, "exported_at":now(), "data":snapshot(&conn,&self.dir)?, "events":crate::repository::all_events(&conn)?});
        let folder = self.dir.join("exports");
        std::fs::create_dir_all(&folder)?;
        let path = folder.join(format!(
            "dayflow-{}-{}.json",
            chrono::Local::now().format("%Y%m%d-%H%M%S"),
            &id()[..8]
        ));
        std::fs::write(&path, serde_json::to_vec_pretty(&value)?)?;
        Ok(path)
    }
}
pub fn settings(conn: &Connection) -> Result<Settings> {
    use rusqlite::OptionalExtension;
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key='preferences'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    value
        .map(|s| serde_json::from_str(&s).map_err(Into::into))
        .unwrap_or_else(|| Ok(Settings::default()))
}
pub fn save_settings(conn: &Connection, s: &Settings) -> Result<()> {
    validate_settings(s)?;
    conn.execute("INSERT INTO settings(key,value,updated_at_utc) VALUES('preferences',?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at_utc=excluded.updated_at_utc", params![serde_json::to_string(s)?,now()])?;
    Ok(())
}
fn validate_settings(s: &Settings) -> Result<()> {
    if ![3, 5, 8].contains(&s.widget_count)
        || !(0.35..=1.0).contains(&s.widget_opacity)
        || !["light", "dark", "system"].contains(&s.theme.as_str())
        || !["high", "medium", "low"].contains(&s.default_priority.as_str())
    {
        bail!("设置值无效");
    }
    for time in [&s.quiet_start, &s.quiet_end, &s.date_reminder_time] {
        crate::repository::validate_time(time)?;
    }
    Ok(())
}
pub fn snapshot(conn: &Connection, dir: &Path) -> Result<Snapshot> {
    Ok(Snapshot {
        tasks: crate::repository::tasks(conn)?,
        categories: crate::repository::categories(conn)?,
        work_records: crate::repository::work_records(conn)?,
        settings: settings(conn)?,
        data_dir: dir.to_string_lossy().into_owned(),
    })
}

fn schema(conn: &Connection) -> Result<Vec<(String, String, String, String)>> {
    Ok(conn.prepare("SELECT type,name,tbl_name,sql FROM sqlite_schema WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY type,name")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, String>(3)?.split_whitespace().collect::<Vec<_>>().join(" "))))?
        .collect::<rusqlite::Result<_>>()?)
}

fn validate_task_data(task: &Task) -> Result<()> {
    if task.id.is_empty()
        || task.title.trim().is_empty()
        || task.title.chars().count() > 500
        || task.description.chars().count() > 20000
        || !["todo", "in_progress", "completed", "cancelled"].contains(&task.status.as_str())
        || !["high", "medium", "low"].contains(&task.priority.as_str())
        || (task.status == "completed") != task.completed_at_utc.is_some()
        || (task.due_time_local.is_some() && task.due_date_local.is_none())
    {
        bail!("备份任务数据无效");
    }
    if let Some(date) = &task.due_date_local {
        crate::repository::validate_date(date)?;
    }
    if let Some(time) = &task.due_time_local {
        crate::repository::validate_time(time)?;
    }
    for timestamp in [
        Some(&task.created_at_utc),
        Some(&task.updated_at_utc),
        task.completed_at_utc.as_ref(),
        task.deleted_at_utc.as_ref(),
        task.due_at_utc.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        chrono::DateTime::parse_from_rfc3339(timestamp).context("备份任务时间无效")?;
    }
    Ok(())
}

fn validate_backup(conn: &Connection, dir: &Path) -> Result<()> {
    let mut expected = Connection::open_in_memory()?;
    Database::migrate(&mut expected)?;
    if schema(conn)? != schema(&expected)? {
        bail!("备份数据库结构无效");
    }
    let mut checks = conn.prepare("PRAGMA integrity_check")?;
    let checks = checks
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if checks != ["ok"] {
        bail!("备份完整性检查失败");
    }
    if conn.prepare("PRAGMA foreign_key_check")?.exists([])? {
        bail!("备份外键检查失败");
    }
    let data = snapshot(conn, dir)?;
    for task in &data.tasks {
        validate_task_data(task)?;
    }
    for record in &data.work_records {
        if let WorkRecord::Manual { log } = record {
            if !["completed", "progress"].contains(&log.kind.as_str())
                || log.content.trim().is_empty()
                || log.content.chars().count() > 20000
            {
                bail!("备份工作记录无效");
            }
            crate::repository::validate_date(&log.local_date)?;
            chrono::DateTime::parse_from_rfc3339(&log.created_at_utc)?;
            chrono::DateTime::parse_from_rfc3339(&log.updated_at_utc)?;
        }
    }
    validate_settings(&data.settings)?;
    for category in &data.categories {
        if category.name.trim().is_empty()
            || category.name.chars().count() > 30
            || category.color.len() != 7
            || !category.color.starts_with('#')
            || !category.color[1..].chars().all(|c| c.is_ascii_hexdigit())
        {
            bail!("备份分类无效");
        }
        chrono::DateTime::parse_from_rfc3339(&category.created_at_utc)?;
    }
    for event in crate::repository::all_events(conn)? {
        chrono::DateTime::parse_from_rfc3339(&event.created_at_utc)?;
        let task: Task = serde_json::from_str(&event.snapshot_json).context("备份任务快照损坏")?;
        validate_task_data(&task)?;
        if task.id != event.task_id || task.title != event.task_title_snapshot {
            bail!("备份任务快照与事件不一致");
        }
    }
    Ok(())
}
