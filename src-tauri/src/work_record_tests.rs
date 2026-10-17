use crate::{db::Database, models::*, reports, repository as repo};
use chrono::Duration;
use rusqlite::{params, Connection};
use std::path::Path;

fn task_input(title: &str) -> TaskInput {
    TaskInput {
        id: None,
        title: title.into(),
        description: String::new(),
        status: "todo".into(),
        priority: "medium".into(),
        category_id: None,
        due_date_local: None,
        due_time_local: None,
        expected_updated_at: None,
    }
}

fn task_update(task: &Task, title: &str) -> TaskInput {
    TaskInput {
        id: Some(task.id.clone()),
        title: title.into(),
        description: task.description.clone(),
        status: task.status.clone(),
        priority: task.priority.clone(),
        category_id: task.category_id.clone(),
        due_date_local: task.due_date_local.clone(),
        due_time_local: task.due_time_local.clone(),
        expected_updated_at: Some(task.updated_at_utc.clone()),
    }
}

fn log_input(content: &str, kind: &str, date: &str, task_id: Option<&str>) -> LogInput {
    LogInput {
        id: None,
        task_id: task_id.map(str::to_owned),
        local_date: date.into(),
        content: content.into(),
        kind: kind.into(),
        expected_updated_at: None,
    }
}

fn at(date: &str, hour: i64) -> String {
    let boundary = reports::boundary(repo::validate_date(date).unwrap()).unwrap();
    (chrono::DateTime::parse_from_rfc3339(&boundary).unwrap() + Duration::hours(hour))
        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}

fn move_latest_event(c: &Connection, task_id: &str, date: &str, hour: i64) -> String {
    // The real mutation creates its event; freeze only its time in this temporary database.
    let event_id: String = c
        .query_row(
            "SELECT id FROM task_events WHERE task_id=?1 ORDER BY rowid DESC LIMIT 1",
            [task_id],
            |row| row.get(0),
        )
        .unwrap();
    c.execute(
        "UPDATE task_events SET created_at_utc=?1 WHERE id=?2",
        params![at(date, hour), event_id],
    )
    .unwrap();
    event_id
}

fn move_log_creation(c: &Connection, log_id: &str, date: &str, hour: i64) {
    c.execute(
        "UPDATE work_logs SET created_at_utc=?1 WHERE id=?2",
        params![at(date, hour), log_id],
    )
    .unwrap();
}

fn content(record: &WorkRecord) -> &str {
    match record {
        WorkRecord::TaskCompletion { content, .. } => content,
        WorkRecord::Manual { log } => &log.content,
    }
}
fn manual_count(report: &reports::Report, kind: &str) -> usize {
    report
        .work_records
        .iter()
        .filter(|record| matches!(record, WorkRecord::Manual { log } if log.kind == kind))
        .count()
}

fn with_db(test: impl FnOnce(&Database)) {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    test(&db);
}

fn create_v1_database(path: &Path) {
    let c = Connection::open(path).unwrap();
    c.execute_batch(include_str!("../migrations/0001_init.sql"))
        .unwrap();
    c.pragma_update(None, "user_version", 1).unwrap();
    c.execute(
        "INSERT INTO work_logs(id,task_id,local_date,content,created_at_utc,updated_at_utc) VALUES('legacy',NULL,'2024-02-29','旧版原始内容',?1,?1)",
        [at("2024-03-01", 9)],
    )
    .unwrap();
}

#[test]
fn planned_task_completion_appears_without_creating_a_manual_record() {
    with_db(|db| {
        let mut c = db.lock().unwrap();
        let mut input = task_input("上周安排、今天交付");
        input.due_date_local = Some("2026-10-05".into());
        let task = repo::save_task(&mut c, input).unwrap();
        move_latest_event(&c, &task.id, "2026-10-05", 9);
        repo::set_status(&mut c, &task.id, "completed").unwrap();
        move_latest_event(&c, &task.id, "2026-10-13", 11);

        assert!(repo::logs(&c).unwrap().is_empty());
        let records = repo::work_records(&c).unwrap();
        assert_eq!(records.len(), 1);
        match &records[0] {
            WorkRecord::TaskCompletion {
                task_id,
                local_date,
                content,
                created_at_utc,
                reopened_at_utc,
                ..
            } => {
                assert_eq!(task_id, &task.id);
                assert_eq!(local_date, "2026-10-13");
                assert_eq!(content, "上周安排、今天交付");
                assert_eq!(created_at_utc, &at("2026-10-13", 11));
                assert!(reopened_at_utc.is_none());
            }
            WorkRecord::Manual { .. } => panic!("任务完成不应生成手动记录"),
        }
        let snapshot = crate::db::snapshot(&c, &db.dir).unwrap();
        assert_eq!(snapshot.work_records.len(), 1);
        let report = reports::generate(&c, "day", "2026-10-13").unwrap();
        assert_eq!(report.work_records.len(), 1);
        assert!(matches!(
            report.work_records[0],
            WorkRecord::TaskCompletion { .. }
        ));
        assert!(reports::generate(&c, "day", "2026-10-05")
            .unwrap()
            .work_records
            .is_empty());
    });
}

#[test]
fn linked_manual_records_remain_independent_and_never_change_task_state() {
    with_db(|db| {
        let mut c = db.lock().unwrap();
        let task = repo::save_task(&mut c, task_input("完成接口开发")).unwrap();
        move_latest_event(&c, &task.id, "2026-10-12", 9);
        let before_events = repo::all_events(&c).unwrap().len();
        repo::save_log(
            &c,
            log_input(
                "临时完成接口排查",
                "completed",
                "2026-10-13",
                Some(&task.id),
            ),
        )
        .unwrap();
        repo::save_log(
            &c,
            log_input("接口联调推进一半", "progress", "2026-10-13", Some(&task.id)),
        )
        .unwrap();
        assert_eq!(repo::task(&c, &task.id).unwrap(), task);
        assert_eq!(repo::all_events(&c).unwrap().len(), before_events);

        repo::set_status(&mut c, &task.id, "completed").unwrap();
        move_latest_event(&c, &task.id, "2026-10-13", 14);
        let records = repo::work_records(&c).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(
            records
                .iter()
                .filter(|record| matches!(record, WorkRecord::TaskCompletion { .. }))
                .count(),
            1
        );
        assert_eq!(repo::logs(&c).unwrap().len(), 2);
        for kind in ["day", "week", "month"] {
            let report = reports::generate(&c, kind, "2026-10-13").unwrap();
            assert_eq!(manual_count(&report, "completed"), 1);
            assert_eq!(manual_count(&report, "progress"), 1);
            assert_eq!(report.work_records.len(), 3);
        }
    });
}

#[test]
fn repeated_completions_keep_daily_history_and_latest_period_snapshot() {
    with_db(|db| {
        let mut c = db.lock().unwrap();
        let task = repo::save_task(&mut c, task_input("第一天交付")).unwrap();
        move_latest_event(&c, &task.id, "2026-10-11", 9);
        repo::set_status(&mut c, &task.id, "completed").unwrap();
        let first = move_latest_event(&c, &task.id, "2026-10-12", 10);
        let task = repo::set_status(&mut c, &task.id, "todo").unwrap();
        let reopen = move_latest_event(&c, &task.id, "2026-10-13", 9);
        let task = repo::save_task(&mut c, task_update(&task, "第二天第一次交付")).unwrap();
        move_latest_event(&c, &task.id, "2026-10-13", 10);
        repo::set_status(&mut c, &task.id, "completed").unwrap();
        move_latest_event(&c, &task.id, "2026-10-13", 11);
        let task = repo::set_status(&mut c, &task.id, "in_progress").unwrap();
        move_latest_event(&c, &task.id, "2026-10-13", 12);
        let task = repo::save_task(&mut c, task_update(&task, "第二天最终交付")).unwrap();
        move_latest_event(&c, &task.id, "2026-10-13", 13);
        repo::set_status(&mut c, &task.id, "completed").unwrap();
        let last = move_latest_event(&c, &task.id, "2026-10-13", 14);
        let task = repo::set_status(&mut c, &task.id, "cancelled").unwrap();
        move_latest_event(&c, &task.id, "2026-10-14", 9);
        let task = repo::save_task(&mut c, task_update(&task, "事后改名")).unwrap();
        move_latest_event(&c, &task.id, "2026-10-14", 10);
        repo::delete_task(&mut c, &task.id, false).unwrap();
        move_latest_event(&c, &task.id, "2026-10-14", 11);

        let records = repo::work_records(&c).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(content(&records[0]), "第二天最终交付");
        assert_eq!(content(&records[1]), "第一天交付");
        for record in &records {
            match record {
                WorkRecord::TaskCompletion {
                    event_id,
                    local_date,
                    reopened_at_utc,
                    ..
                } if local_date == "2026-10-12" => {
                    assert_eq!(event_id, &first);
                    assert_eq!(
                        reopened_at_utc.as_deref(),
                        Some(at("2026-10-13", 9).as_str())
                    );
                }
                WorkRecord::TaskCompletion {
                    event_id,
                    reopened_at_utc,
                    ..
                } => {
                    assert_eq!(event_id, &last);
                    assert!(reopened_at_utc.is_none(), "取消不等于重新开启");
                }
                WorkRecord::Manual { .. } => panic!("没有保存过手动记录"),
            }
        }
        assert!(repo::all_events(&c)
            .unwrap()
            .iter()
            .any(|event| event.id == reopen));
        let day = reports::generate(&c, "day", "2026-10-13").unwrap();
        assert_eq!(day.work_records.len(), 1);
        assert_eq!(content(&day.work_records[0]), "第二天最终交付");
        assert_eq!(
            repo::all_events(&c)
                .unwrap()
                .iter()
                .filter(|event| event.task_id == task.id && event.event_type == "completed")
                .count(),
            3
        );
        for kind in ["week", "month"] {
            let report = reports::generate(&c, kind, "2026-10-13").unwrap();
            assert_eq!(report.work_records.len(), 1);
            assert_eq!(content(&report.work_records[0]), "第二天最终交付");
        }
    });
}

#[test]
fn legacy_cancel_event_named_reopened_does_not_mark_a_completion_reopened() {
    with_db(|db| {
        let mut c = db.lock().unwrap();
        let task = repo::save_task(&mut c, task_input("后来取消的成果")).unwrap();
        move_latest_event(&c, &task.id, "2026-10-12", 9);
        repo::set_status(&mut c, &task.id, "completed").unwrap();
        move_latest_event(&c, &task.id, "2026-10-13", 10);
        repo::set_status(&mut c, &task.id, "cancelled").unwrap();
        let cancelled = move_latest_event(&c, &task.id, "2026-10-14", 11);
        // V1 labeled completed -> cancelled as "reopened". Its status value is decisive.
        c.execute(
            "UPDATE task_events SET event_type='reopened' WHERE id=?1",
            [cancelled],
        )
        .unwrap();
        let records = repo::work_records(&c).unwrap();
        assert_eq!(records.len(), 1);
        match &records[0] {
            WorkRecord::TaskCompletion {
                reopened_at_utc, ..
            } => {
                assert!(reopened_at_utc.is_none());
            }
            WorkRecord::Manual { .. } => panic!("应展示任务完成历史"),
        }
        repo::set_status(&mut c, &task.id, "todo").unwrap();
        move_latest_event(&c, &task.id, "2026-10-15", 9);
        match &repo::work_records(&c).unwrap()[0] {
            WorkRecord::TaskCompletion {
                reopened_at_utc, ..
            } => {
                assert_eq!(
                    reopened_at_utc.as_deref(),
                    Some(at("2026-10-15", 9).as_str())
                );
            }
            WorkRecord::Manual { .. } => panic!("应展示任务完成历史"),
        }
    });
}

#[test]
fn work_records_sort_by_selected_day_then_original_recording_time() {
    with_db(|db| {
        let mut c = db.lock().unwrap();
        let task = repo::save_task(&mut c, task_input("当天十点完成")).unwrap();
        repo::set_status(&mut c, &task.id, "completed").unwrap();
        move_latest_event(&c, &task.id, "2026-10-14", 10);
        for (text, kind, local_date, created_date, hour) in [
            ("当天九点进展", "progress", "2026-10-14", "2026-10-14", 9),
            (
                "当天十二点手动完成",
                "completed",
                "2026-10-14",
                "2026-10-14",
                12,
            ),
            (
                "后来补记十二日",
                "completed",
                "2026-10-12",
                "2026-10-16",
                15,
            ),
            ("十五日所属记录", "progress", "2026-10-15", "2026-10-11", 8),
        ] {
            let log = repo::save_log(&c, log_input(text, kind, local_date, None)).unwrap();
            move_log_creation(&c, &log.id, created_date, hour);
        }
        let records = repo::work_records(&c).unwrap();
        assert_eq!(
            records.iter().map(content).collect::<Vec<_>>(),
            vec![
                "十五日所属记录",
                "当天十二点手动完成",
                "当天十点完成",
                "当天九点进展",
                "后来补记十二日",
            ]
        );
        let historical = reports::generate(&c, "day", "2026-10-12").unwrap();
        assert_eq!(manual_count(&historical, "completed"), 1);
        assert_eq!(historical.work_records.len(), 1);
        assert_eq!(content(&historical.work_records[0]), "后来补记十二日");
        let recording_day = reports::generate(&c, "day", "2026-10-16").unwrap();
        assert!(recording_day.work_records.is_empty());
    });
}

#[test]
fn manual_kind_is_required_validated_and_editing_preserves_conflict_detection() {
    with_db(|db| {
        let c = db.lock().unwrap();
        assert!(serde_json::from_value::<LogInput>(serde_json::json!({
            "id": null,
            "task_id": null,
            "local_date": "2026-10-13",
            "content": "未指定类型",
            "expected_updated_at": null
        }))
        .is_err());
        for kind in ["", "done", "Completed", " completed "] {
            assert!(repo::save_log(&c, log_input("非法类型", kind, "2026-10-13", None)).is_err());
        }
        let log =
            repo::save_log(&c, log_input("已完成事项", "completed", "2026-10-13", None)).unwrap();
        c.execute(
            "UPDATE work_logs SET updated_at_utc=?1 WHERE id=?2",
            params![at("2026-10-13", 9), log.id],
        )
        .unwrap();
        let mut edit = log_input("调整为历史进展", "progress", "2026-10-12", None);
        edit.id = Some(log.id.clone());
        edit.expected_updated_at = Some(at("2026-10-13", 9));
        let updated = repo::save_log(&c, edit.clone()).unwrap();
        assert_eq!(updated.kind, "progress");
        assert_eq!(updated.local_date, "2026-10-12");
        assert_eq!(updated.created_at_utc, log.created_at_utc);
        assert!(repo::save_log(&c, edit).is_err());
        let logs = repo::logs(&c).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].content, "调整为历史进展");
        assert_eq!(
            manual_count(
                &reports::generate(&c, "day", "2026-10-12").unwrap(),
                "progress"
            ),
            1
        );
        assert_eq!(
            manual_count(
                &reports::generate(&c, "day", "2026-10-13").unwrap(),
                "completed"
            ),
            0
        );
    });
}

#[test]
fn v1_database_upgrade_preserves_original_logs_as_progress_and_is_repeatable() {
    let dir = tempfile::tempdir().unwrap();
    create_v1_database(&dir.path().join("dayflow.db"));
    {
        let db = Database::open(dir.path()).unwrap();
        let mut c = db.lock().unwrap();
        Database::migrate(&mut c).unwrap();
        let version: i64 = c
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
        let logs = repo::logs(&c).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].id, "legacy");
        assert_eq!(logs[0].kind, "progress");
        assert_eq!(logs[0].content, "旧版原始内容");
        assert_eq!(logs[0].local_date, "2024-02-29");
        assert_eq!(logs[0].created_at_utc, at("2024-03-01", 9));
        assert_eq!(logs[0].updated_at_utc, at("2024-03-01", 9));
        assert_eq!(logs[0].task_id, None);
        assert!(repo::all_events(&c).unwrap().is_empty());
        let report = reports::generate(&c, "day", "2024-02-29").unwrap();
        assert_eq!(manual_count(&report, "progress"), 1);
        assert_eq!(manual_count(&report, "completed"), 0);
    }
    let reopened = Database::open(dir.path()).unwrap();
    assert_eq!(
        repo::logs(&reopened.lock().unwrap()).unwrap()[0].kind,
        "progress"
    );
}

#[test]
fn restoring_v1_backup_migrates_a_copy_and_is_immediately_writable() {
    let source_dir = tempfile::tempdir().unwrap();
    let source_path = source_dir.path().join("version-one.db");
    create_v1_database(&source_path);
    let original_bytes = std::fs::read(&source_path).unwrap();
    let live_dir = tempfile::tempdir().unwrap();
    let db = Database::open(live_dir.path()).unwrap();
    {
        let mut c = db.lock().unwrap();
        repo::save_task(&mut c, task_input("恢复前应被安全备份")).unwrap();
    }
    let safety = db.restore(&source_path).unwrap();
    let preserved = Connection::open(&safety).unwrap();
    assert_eq!(
        repo::tasks(&preserved).unwrap()[0].title,
        "恢复前应被安全备份"
    );
    {
        let mut c = db.lock().unwrap();
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert_eq!(repo::logs(&c).unwrap()[0].kind, "progress");
        assert!(repo::tasks(&c).unwrap().is_empty());
        let task = repo::save_task(&mut c, task_input("恢复后立即完成")).unwrap();
        repo::set_status(&mut c, &task.id, "completed").unwrap();
        repo::save_log(
            &c,
            log_input("恢复后临时完成", "completed", "2026-10-13", Some(&task.id)),
        )
        .unwrap();
        assert_eq!(repo::work_records(&c).unwrap().len(), 3);
    }
    assert_eq!(std::fs::read(&source_path).unwrap(), original_bytes);
    let original = Connection::open(&source_path).unwrap();
    assert_eq!(
        original
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    drop(db);
    let reopened = Database::open(live_dir.path()).unwrap();
    let c = reopened.lock().unwrap();
    assert_eq!(repo::logs(&c).unwrap().len(), 2);
    assert_eq!(repo::tasks(&c).unwrap()[0].status, "completed");
}

#[test]
fn restoring_v2_backup_preserves_types_and_allows_immediate_edits() {
    let source_dir = tempfile::tempdir().unwrap();
    let backup;
    {
        let source = Database::open(source_dir.path()).unwrap();
        {
            let mut c = source.lock().unwrap();
            let task = repo::save_task(&mut c, task_input("备份中的自动成果")).unwrap();
            repo::set_status(&mut c, &task.id, "completed").unwrap();
            repo::save_log(
                &c,
                log_input("备份中的手动成果", "completed", "2026-10-13", None),
            )
            .unwrap();
            repo::save_log(
                &c,
                log_input("备份中的进展", "progress", "2026-10-13", None),
            )
            .unwrap();
        }
        backup = source.backup("v2-records").unwrap();
    }
    let original_bytes = std::fs::read(&backup).unwrap();
    let live_dir = tempfile::tempdir().unwrap();
    let db = Database::open(live_dir.path()).unwrap();
    db.restore(&backup).unwrap();
    {
        let mut c = db.lock().unwrap();
        assert_eq!(repo::work_records(&c).unwrap().len(), 3);
        let log = repo::logs(&c)
            .unwrap()
            .into_iter()
            .find(|log| log.kind == "completed")
            .unwrap();
        let mut edit = log_input("恢复后调整为进展", "progress", "2026-10-12", None);
        edit.id = Some(log.id);
        edit.expected_updated_at = Some(log.updated_at_utc);
        assert_eq!(repo::save_log(&c, edit).unwrap().kind, "progress");
        let task = repo::tasks(&c).unwrap().pop().unwrap();
        repo::set_status(&mut c, &task.id, "todo").unwrap();
        assert_eq!(repo::task(&c, &task.id).unwrap().status, "todo");
        assert_eq!(
            manual_count(
                &reports::generate(&c, "day", "2026-10-12").unwrap(),
                "progress"
            ),
            1
        );
    }
    assert_eq!(std::fs::read(&backup).unwrap(), original_bytes);
    drop(db);
    let reopened = Database::open(live_dir.path()).unwrap();
    assert_eq!(
        repo::logs(&reopened.lock().unwrap())
            .unwrap()
            .iter()
            .filter(|log| log.kind == "progress")
            .count(),
        2
    );
}

#[test]
fn invalid_and_future_backups_never_replace_current_work_records() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    {
        let mut c = db.lock().unwrap();
        let task = repo::save_task(&mut c, task_input("不可被坏备份覆盖")).unwrap();
        repo::set_status(&mut c, &task.id, "completed").unwrap();
        repo::save_log(
            &c,
            log_input("保留手动记录", "completed", "2026-10-13", None),
        )
        .unwrap();
    }
    let before = {
        let c = db.lock().unwrap();
        serde_json::to_value(crate::db::snapshot(&c, &db.dir).unwrap()).unwrap()
    };
    let invalid = dir.path().join("invalid.db");
    std::fs::write(&invalid, b"not a sqlite database").unwrap();
    let future = dir.path().join("future.db");
    {
        let c = Connection::open(&future).unwrap();
        c.pragma_update(None, "user_version", 3).unwrap();
    }
    let missing_schema = dir.path().join("missing-schema.db");
    {
        let c = Connection::open(&missing_schema).unwrap();
        c.pragma_update(None, "user_version", 2).unwrap();
    }
    let bad_snapshot = db.backup("bad-event-snapshot").unwrap();
    {
        let c = Connection::open(&bad_snapshot).unwrap();
        c.execute("UPDATE task_events SET snapshot_json='invalid-json'", [])
            .unwrap();
    }
    for path in [&invalid, &future, &missing_schema, &bad_snapshot] {
        assert!(db.restore(path).is_err(), "不应接受 {}", path.display());
        let c = db.lock().unwrap();
        assert_eq!(
            serde_json::to_value(crate::db::snapshot(&c, &db.dir).unwrap()).unwrap(),
            before
        );
        assert_eq!(repo::tasks(&c).unwrap()[0].status, "completed");
    }
}

#[test]
fn json_export_uses_version_two_and_the_shared_work_record_projection() {
    with_db(|db| {
        {
            let mut c = db.lock().unwrap();
            let task = repo::save_task(&mut c, task_input("导出自动成果")).unwrap();
            repo::set_status(&mut c, &task.id, "completed").unwrap();
            repo::save_log(
                &c,
                log_input("导出手动完成", "completed", "2026-10-13", None),
            )
            .unwrap();
            repo::save_log(
                &c,
                log_input("导出工作进展", "progress", "2026-10-13", None),
            )
            .unwrap();
        }
        let path = db.export().unwrap();
        let exported: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(exported["format"], "DayFlow");
        assert_eq!(exported["version"], 2);
        let records = exported["data"]["work_records"].as_array().unwrap();
        assert_eq!(records.len(), 3);
        assert!(exported["data"].get("logs").is_none());
        assert!(exported["data"].get("completion_events").is_none());
        let mut kinds = records
            .iter()
            .filter(|record| record["source"] == "manual")
            .map(|record| record["kind"].as_str().unwrap())
            .collect::<Vec<_>>();
        kinds.sort_unstable();
        assert_eq!(kinds, vec!["completed", "progress"]);
        assert_eq!(
            exported["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|event| event["event_type"] == "completed")
                .count(),
            1
        );
    });
}

#[test]
fn report_ranges_include_full_next_calendar_period_across_years_and_leap_days() {
    with_db(|db| {
        let c = db.lock().unwrap();
        for (kind, date, start, end, next_start, next_end) in [
            (
                "day",
                "2026-12-31",
                "2026-12-31",
                "2026-12-31",
                "2027-01-01",
                "2027-01-01",
            ),
            (
                "day",
                "2024-02-28",
                "2024-02-28",
                "2024-02-28",
                "2024-02-29",
                "2024-02-29",
            ),
            (
                "day",
                "2024-02-29",
                "2024-02-29",
                "2024-02-29",
                "2024-03-01",
                "2024-03-01",
            ),
            (
                "week",
                "2027-01-03",
                "2026-12-28",
                "2027-01-03",
                "2027-01-04",
                "2027-01-10",
            ),
            (
                "week",
                "2024-02-29",
                "2024-02-26",
                "2024-03-03",
                "2024-03-04",
                "2024-03-10",
            ),
            (
                "month",
                "2026-12-31",
                "2026-12-01",
                "2026-12-31",
                "2027-01-01",
                "2027-01-31",
            ),
            (
                "month",
                "2024-01-30",
                "2024-01-01",
                "2024-01-31",
                "2024-02-01",
                "2024-02-29",
            ),
            (
                "month",
                "2023-01-30",
                "2023-01-01",
                "2023-01-31",
                "2023-02-01",
                "2023-02-28",
            ),
        ] {
            let report = reports::generate(&c, kind, date).unwrap();
            assert_eq!(report.start, start, "{kind} {date}");
            assert_eq!(report.end, end, "{kind} {date}");
            assert_eq!(report.next_start, next_start, "{kind} {date}");
            assert_eq!(report.next_end, next_end, "{kind} {date}");
            let json = serde_json::to_value(report).unwrap();
            let mut fields = json
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>();
            fields.sort_unstable();
            assert_eq!(
                fields,
                [
                    "end",
                    "kind",
                    "next_end",
                    "next_start",
                    "start",
                    "unfinished",
                    "upcoming",
                    "work_records"
                ]
            );
        }
    });
}

#[test]
fn next_period_tasks_use_due_boundaries_and_the_status_known_at_period_end() {
    for (kind, date) in [
        ("day", "2026-12-31"),
        ("week", "2027-01-03"),
        ("month", "2024-01-30"),
    ] {
        with_db(|db| {
            let mut c = db.lock().unwrap();
            let (start, end) = reports::period(kind, repo::validate_date(date).unwrap()).unwrap();
            let (next_start, next_end) = reports::period(kind, end).unwrap();
            let start_date = start.to_string();
            let next_date = next_start.to_string();
            let create = |c: &mut Connection, title: &str, due_date: Option<String>| {
                let mut input = task_input(title);
                input.due_date_local = due_date;
                let task = repo::save_task(c, input).unwrap();
                move_latest_event(c, &task.id, &start_date, 9);
                task
            };

            create(
                &mut c,
                "本周期及逾期未完成",
                Some((next_start - Duration::days(1)).to_string()),
            );
            let first = create(&mut c, "下周期首日", Some(next_date.clone()));
            let last = create(
                &mut c,
                "下周期末日",
                Some((next_end - Duration::days(1)).to_string()),
            );
            create(&mut c, "下下周期首日", Some(next_end.to_string()));
            create(&mut c, "无截止日期", None);
            create(
                &mut c,
                "更远的任务",
                Some((next_end + Duration::days(30)).to_string()),
            );
            let changed = create(&mut c, "期末的原计划", Some(next_date.clone()));
            let progress = create(&mut c, "期末进行中", Some(next_date.clone()));
            repo::set_status(&mut c, &progress.id, "in_progress").unwrap();
            move_latest_event(&c, &progress.id, &start_date, 11);

            // Exactly at the next period's midnight, these changes no longer belong to this report.
            repo::set_status(&mut c, &first.id, "completed").unwrap();
            move_latest_event(&c, &first.id, &next_date, 0);
            repo::delete_task(&mut c, &last.id, false).unwrap();
            move_latest_event(&c, &last.id, &next_date, 0);
            let mut edit = task_update(&changed, "期末的原计划");
            edit.due_date_local = Some(next_end.to_string());
            repo::save_task(&mut c, edit).unwrap();
            move_latest_event(&c, &changed.id, &next_date, 0);
            let born_later = create(&mut c, "下周期才新建", Some(next_date.clone()));
            move_latest_event(&c, &born_later.id, &next_date, 0);

            let completed = create(&mut c, "期末已完成", Some(next_date.clone()));
            repo::set_status(&mut c, &completed.id, "completed").unwrap();
            move_latest_event(&c, &completed.id, &start_date, 12);
            repo::set_status(&mut c, &completed.id, "todo").unwrap();
            move_latest_event(&c, &completed.id, &next_date, 0);
            let cancelled = create(&mut c, "期末已取消", Some(next_date.clone()));
            repo::set_status(&mut c, &cancelled.id, "cancelled").unwrap();
            move_latest_event(&c, &cancelled.id, &start_date, 12);
            let deleted = create(&mut c, "期末已删除", Some(next_date.clone()));
            repo::delete_task(&mut c, &deleted.id, false).unwrap();
            move_latest_event(&c, &deleted.id, &start_date, 12);

            let report = reports::generate(&c, kind, date).unwrap();
            assert_eq!(
                report
                    .unfinished
                    .iter()
                    .map(|task| task.title.as_str())
                    .collect::<std::collections::BTreeSet<_>>(),
                [
                    "本周期及逾期未完成",
                    "下周期首日",
                    "下周期末日",
                    "下下周期首日",
                    "无截止日期",
                    "更远的任务",
                    "期末的原计划",
                    "期末进行中"
                ]
                .into_iter()
                .collect(),
                "{kind}"
            );
            assert_eq!(
                report
                    .upcoming
                    .iter()
                    .map(|task| task.title.as_str())
                    .collect::<std::collections::BTreeSet<_>>(),
                ["下周期首日", "下周期末日", "期末的原计划", "期末进行中"]
                    .into_iter()
                    .collect(),
                "{kind}"
            );
            assert!(report
                .upcoming
                .iter()
                .all(|upcoming| report.unfinished.iter().any(|task| task.id == upcoming.id)));
            assert_eq!(
                report
                    .upcoming
                    .iter()
                    .find(|task| task.id == changed.id)
                    .unwrap()
                    .due_date_local
                    .as_deref(),
                Some(next_date.as_str())
            );
            assert_eq!(
                report
                    .upcoming
                    .iter()
                    .find(|task| task.id == progress.id)
                    .unwrap()
                    .status,
                "in_progress"
            );
            assert_eq!(report.work_records.len(), 1, "截止边界的完成不能计入本周期");
            assert_eq!(content(&report.work_records[0]), "期末已完成");
        });
    }
}
