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
        if version > 1 {
            bail!("数据库由更高版本创建，不能用旧版打开");
        }
        if version < 1 {
            let tx = conn.transaction()?;
            tx.execute_batch(include_str!("../migrations/0001_init.sql"))?;
            tx.pragma_update(None, "user_version", 1)?;
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
        let check: String = source.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if check != "ok" {
            bail!("备份完整性检查失败");
        }
        let version: i64 = source.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version != 1 {
            bail!("不是受支持的 DayFlow 数据库备份");
        }
        // Validate all required columns before touching the live database.
        source.prepare("SELECT id,title,description,status,priority,category_id,due_date_local,due_time_local,due_at_utc,created_at_utc,updated_at_utc,completed_at_utc,deleted_at_utc FROM tasks")?;
        source.prepare("SELECT id,task_id,event_type,old_value,new_value,task_title_snapshot,snapshot_json,created_at_utc FROM task_events")?;
        source.prepare(
            "SELECT id,task_id,local_date,content,created_at_utc,updated_at_utc FROM work_logs",
        )?;
        source.prepare("SELECT id,name,color,created_at_utc FROM categories")?;
        source.prepare("SELECT key,value,updated_at_utc FROM settings")?;
        source.prepare("SELECT task_id,due_key,sent_at_utc FROM notification_deliveries")?;
        if source.prepare("PRAGMA foreign_key_check")?.exists([])? {
            bail!("备份外键检查失败");
        }
        snapshot(&source, &self.dir)?;
        for e in crate::repository::all_events(&source)? {
            let _: Task = serde_json::from_str(&e.snapshot_json).context("备份任务快照损坏")?;
        }
        let mut conn = self.lock()?;
        let safety = Self::backup_conn(&conn, &self.dir, "before-restore")?;
        rusqlite::backup::Backup::new(&source, &mut conn)?
            .run_to_completion(128, Duration::from_millis(5), None)
            .context("恢复失败，原数据库已保留安全备份")?;
        conn.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        Ok(safety)
    }
    pub fn export(&self) -> Result<PathBuf> {
        let conn = self.lock()?;
        let value = serde_json::json!({"format":"DayFlow", "version":1, "exported_at":now(), "data":snapshot(&conn,&self.dir)?, "events":crate::repository::all_events(&conn)?});
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
    if !["desktop", "topmost"].contains(&s.widget_mode.as_str())
        || ![3, 5, 8].contains(&s.widget_count)
        || !(0.35..=1.0).contains(&s.widget_opacity)
        || !["light", "dark", "system"].contains(&s.theme.as_str())
        || !["high", "medium", "low"].contains(&s.default_priority.as_str())
    {
        bail!("设置值无效");
    }
    for time in [&s.quiet_start, &s.quiet_end, &s.date_reminder_time] {
        crate::repository::validate_time(time)?;
    }
    conn.execute("INSERT INTO settings(key,value,updated_at_utc) VALUES('preferences',?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at_utc=excluded.updated_at_utc", params![serde_json::to_string(s)?,now()])?;
    Ok(())
}
pub fn snapshot(conn: &Connection, dir: &Path) -> Result<Snapshot> {
    Ok(Snapshot {
        tasks: crate::repository::tasks(conn)?,
        categories: crate::repository::categories(conn)?,
        logs: crate::repository::logs(conn)?,
        settings: settings(conn)?,
        data_dir: dir.to_string_lossy().into_owned(),
        completion_events: conn.prepare("SELECT task_id,created_at_utc FROM task_events WHERE event_type='completed' ORDER BY created_at_utc")?.query_map([],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?,
    })
}
