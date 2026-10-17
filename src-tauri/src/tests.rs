use crate::{db::Database, models::*, reports, repository as repo};
use chrono::{Duration, NaiveDate};
use rusqlite::params;

fn input(title: &str) -> TaskInput {
    TaskInput {
        id: None,
        title: title.into(),
        description: "".into(),
        status: "todo".into(),
        priority: "medium".into(),
        category_id: None,
        due_date_local: None,
        due_time_local: None,
        expected_updated_at: None,
    }
}
fn update(t: &Task) -> TaskInput {
    TaskInput {
        id: Some(t.id.clone()),
        title: t.title.clone(),
        description: t.description.clone(),
        status: t.status.clone(),
        priority: t.priority.clone(),
        category_id: t.category_id.clone(),
        due_date_local: t.due_date_local.clone(),
        due_time_local: t.due_time_local.clone(),
        expected_updated_at: Some(t.updated_at_utc.clone()),
    }
}
fn with_db(f: impl FnOnce(&Database)) {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    f(&db);
}

fn completion_title(report: &reports::Report) -> &str {
    report
        .work_records
        .iter()
        .find_map(|record| match record {
            WorkRecord::TaskCompletion { content, .. } => Some(content.as_str()),
            WorkRecord::Manual { .. } => None,
        })
        .unwrap()
}
fn completion_count(report: &reports::Report) -> usize {
    report
        .work_records
        .iter()
        .filter(|record| matches!(record, WorkRecord::TaskCompletion { .. }))
        .count()
}
fn manual_count(report: &reports::Report, kind: &str) -> usize {
    report
        .work_records
        .iter()
        .filter(|record| matches!(record, WorkRecord::Manual { log } if log.kind == kind))
        .count()
}

#[test]
fn task_lifecycle_and_history() {
    with_db(|db| {
        let mut c = db.lock().unwrap();
        let t = repo::save_task(&mut c, input("  完成项目代码优化  ")).unwrap();
        assert_eq!(t.title, "完成项目代码优化");
        assert_eq!(t.status, "todo");
        let mut full = update(&t);
        full.description = "实现事务".into();
        full.category_id = Some("work".into());
        full.due_date_local = Some("2026-10-17".into());
        full.due_time_local = Some("18:00".into());
        full.priority = "high".into();
        let t = repo::save_task(&mut c, full).unwrap();
        assert!(t.due_at_utc.is_some());
        let t = repo::set_status(&mut c, &t.id, "completed").unwrap();
        assert!(t.completed_at_utc.is_some());
        repo::set_status(&mut c, &t.id, "completed").unwrap();
        assert_eq!(
            repo::all_events(&c)
                .unwrap()
                .iter()
                .filter(|e| e.event_type == "completed")
                .count(),
            1
        );
        let t = repo::set_status(&mut c, &t.id, "todo").unwrap();
        assert!(t.completed_at_utc.is_none());
        repo::set_status(&mut c, &t.id, "cancelled").unwrap();
        repo::delete_task(&mut c, &t.id, false).unwrap();
        repo::delete_task(&mut c, &t.id, false).unwrap();
        assert!(repo::task(&c, &t.id).unwrap().deleted_at_utc.is_some());
        repo::delete_task(&mut c, &t.id, true).unwrap();
        assert!(repo::task(&c, &t.id).unwrap().deleted_at_utc.is_none());
        let events = repo::all_events(&c).unwrap();
        for kind in [
            "created",
            "due_changed",
            "priority_changed",
            "completed",
            "reopened",
            "status_changed",
            "deleted",
            "restored",
        ] {
            assert!(
                events.iter().any(|e| e.event_type == kind),
                "missing {kind}"
            );
        }
    });
}
#[test]
fn validation_and_concurrency() {
    with_db(|db| {
        let mut c = db.lock().unwrap();
        for title in ["", "  ", &"中".repeat(501)] {
            assert!(repo::save_task(&mut c, input(title)).is_err());
        }
        assert!(repo::save_task(&mut c, input(&"中".repeat(500))).is_ok());
        for date in ["2025-02-29", "2026-13-01", "2026-1-01", "1899-01-01"] {
            let mut i = input("错误日期");
            i.due_date_local = Some(date.into());
            assert!(repo::save_task(&mut c, i).is_err());
        }
        let mut i = input("只有时刻");
        i.due_time_local = Some("12:00".into());
        assert!(repo::save_task(&mut c, i).is_err());
        let mut i = input("错误时刻");
        i.due_date_local = Some("2026-10-17".into());
        i.due_time_local = Some("25:00".into());
        assert!(repo::save_task(&mut c, i).is_err());
        let t = repo::save_task(&mut c, input("并发")).unwrap();
        repo::set_status(&mut c, &t.id, "in_progress").unwrap();
        assert!(repo::save_task(&mut c, update(&t)).is_err());
        let mut i = input("外键");
        i.category_id = Some("missing".into());
        assert!(repo::save_task(&mut c, i).is_err());
        assert!(!repo::tasks(&c).unwrap().iter().any(|t| t.title == "外键"));
    });
}
#[test]
fn independent_and_historical_logs() {
    with_db(|db| {
        let c = db.lock().unwrap();
        let l = repo::save_log(
            &c,
            LogInput {
                id: None,
                kind: "progress".into(),
                task_id: None,
                local_date: "2024-02-29".into(),
                content: "帮助同事排查异常".into(),
                expected_updated_at: None,
            },
        )
        .unwrap();
        assert_eq!(l.local_date, "2024-02-29");
        let mut i = LogInput {
            id: Some(l.id.clone()),
            kind: "progress".into(),
            task_id: None,
            local_date: "2024-02-28".into(),
            content: "修改历史日志".into(),
            expected_updated_at: Some(l.updated_at_utc.clone()),
        };
        repo::save_log(&c, i.clone()).unwrap();
        assert!(repo::save_log(&c, i.clone()).is_err());
        i.id = None;
        i.content = " ".into();
        assert!(repo::save_log(&c, i).is_err());
        assert_eq!(repo::logs(&c).unwrap().len(), 1);
    });
}
#[test]
fn calendar_boundaries() {
    let d = |s| repo::validate_date(s).unwrap();
    assert_eq!(
        reports::period("week", d("2027-01-03")).unwrap(),
        (d("2026-12-28"), d("2027-01-04"))
    );
    assert_eq!(
        reports::period("week", d("2027-01-04")).unwrap(),
        (d("2027-01-04"), d("2027-01-11"))
    );
    assert_eq!(
        reports::period("month", d("2024-02-29")).unwrap(),
        (d("2024-02-01"), d("2024-03-01"))
    );
    assert_eq!(
        reports::period("month", d("2026-12-31")).unwrap(),
        (d("2026-12-01"), d("2027-01-01"))
    );
}
// Freeze event dates through SQL only in tests, without inserting demo data in the app.
fn move_events(c: &rusqlite::Connection, task_id: &str, date: &str) {
    c.execute(
        "UPDATE task_events SET created_at_utc=?1 WHERE task_id=?2",
        params![
            reports::boundary(repo::validate_date(date).unwrap()).unwrap(),
            task_id
        ],
    )
    .unwrap();
}
#[test]
fn report_uses_completion_snapshots_after_reopen_rename_delete() {
    with_db(|db| {
        let mut c = db.lock().unwrap();
        let t = repo::save_task(&mut c, input("历史标题")).unwrap();
        let t = repo::set_status(&mut c, &t.id, "completed").unwrap();
        move_events(&c, &t.id, "2024-02-29");
        let t = repo::set_status(&mut c, &t.id, "in_progress").unwrap();
        let mut i = update(&t);
        i.title = "改名后的标题".into();
        repo::save_task(&mut c, i).unwrap();
        repo::delete_task(&mut c, &t.id, false).unwrap();
        let r = reports::generate(&c, "day", "2024-02-29").unwrap();
        assert_eq!(completion_count(&r), 1);
        assert_eq!(completion_title(&r), "历史标题");
        assert!(r.unfinished.is_empty());
        let empty = reports::generate(&c, "day", "2024-03-01").unwrap();
        assert!(empty.work_records.is_empty());
    });
}
#[test]
fn report_current_and_previous_day_progress_logs_and_unique_completions() {
    with_db(|db| {
        let mut c = db.lock().unwrap();
        let date = chrono::Local::now().date_naive();
        let yesterday = date - Duration::days(1);
        let old = repo::save_task(&mut c, input("昨天完成")).unwrap();
        repo::set_status(&mut c, &old.id, "completed").unwrap();
        move_events(&c, &old.id, &yesterday.to_string());
        let t = repo::save_task(&mut c, input("今天完成两次")).unwrap();
        repo::set_status(&mut c, &t.id, "completed").unwrap();
        repo::set_status(&mut c, &t.id, "todo").unwrap();
        repo::set_status(&mut c, &t.id, "completed").unwrap();
        let pending = repo::save_task(&mut c, input("进行中")).unwrap();
        repo::set_status(&mut c, &pending.id, "in_progress").unwrap();
        repo::save_log(
            &c,
            LogInput {
                id: None,
                kind: "progress".into(),
                task_id: Some(pending.id.clone()),
                local_date: date.to_string(),
                content: "完成接口联调".into(),
                expected_updated_at: None,
            },
        )
        .unwrap();
        let r = reports::generate(&c, "day", &date.to_string()).unwrap();
        assert_eq!(completion_count(&r), 1);
        assert_eq!(manual_count(&r, "progress"), 1);
        assert_eq!(
            repo::all_events(&c)
                .unwrap()
                .iter()
                .filter(|event| { event.task_id == t.id && event.event_type == "completed" })
                .count(),
            2
        );
        assert_eq!(r.unfinished.len(), 1);
        assert_eq!(r.unfinished[0].status, "in_progress");
        assert_eq!(r.unfinished[0].title, "进行中");
        assert_eq!(
            completion_count(&reports::generate(&c, "day", &yesterday.to_string()).unwrap()),
            1
        );
    });
}
#[test]
fn empty_reports_have_no_invented_data() {
    with_db(|db| {
        let c = db.lock().unwrap();
        for kind in ["day", "week", "month"] {
            let r = reports::generate(&c, kind, "2026-10-17").unwrap();
            assert!(r.work_records.is_empty());
            assert!(r.unfinished.is_empty());
            assert!(r.upcoming.is_empty());
        }
    });
}
#[test]
fn persistence_backup_restore_and_migration() {
    let dir = tempfile::tempdir().unwrap();
    let task_id;
    {
        let db = Database::open(dir.path()).unwrap();
        let mut c = db.lock().unwrap();
        task_id = repo::save_task(&mut c, input("重启持久化")).unwrap().id;
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(repo::tasks(&db.lock().unwrap()).unwrap().len(), 1);
    let backup = db.backup("test").unwrap();
    {
        let mut c = db.lock().unwrap();
        repo::set_status(&mut c, &task_id, "completed").unwrap();
    }
    let safety = db.restore(&backup).unwrap();
    assert!(safety.exists());
    assert_eq!(
        repo::task(&db.lock().unwrap(), &task_id).unwrap().status,
        "todo"
    );
    let invalid = dir.path().join("invalid.db");
    std::fs::write(&invalid, b"invalid").unwrap();
    assert!(db.restore(&invalid).is_err());
    assert_eq!(repo::tasks(&db.lock().unwrap()).unwrap().len(), 1);
    let export = db.export().unwrap();
    let data: serde_json::Value = serde_json::from_slice(&std::fs::read(export).unwrap()).unwrap();
    assert_eq!(data["data"]["tasks"][0]["title"], "重启持久化");
}
#[test]
fn categories_and_settings_validate_and_persist() {
    with_db(|db| {
        let c = db.lock().unwrap();
        assert_eq!(repo::categories(&c).unwrap().len(), 4);
        repo::save_category(&c, None, "自定义", "#abcd12").unwrap();
        assert!(repo::save_category(&c, None, "自定义", "#abcd12").is_err());
        assert!(repo::save_category(&c, None, "非法", "red").is_err());
        let mut s = Settings::default();
        s.widget_mode = "topmost".into();
        s.widget_count = 8;
        crate::db::save_settings(&c, &s).unwrap();
        assert_eq!(crate::db::settings(&c).unwrap().widget_count, 8);
        s.widget_count = 9;
        assert!(crate::db::save_settings(&c, &s).is_err());
    });
}
#[test]
fn fixed_offsets_do_not_change_period_semantics() {
    use chrono::TimeZone;
    let d = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
    let east = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
    let start = east
        .from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap())
        .unwrap();
    assert_eq!(
        start.with_timezone(&chrono::Utc).to_string(),
        "2024-02-28 16:00:00 UTC"
    );
}

#[test]
fn daylight_saving_boundaries_are_calendar_based() {
    let zone = chrono_tz::America::New_York;
    for (date, hours) in [("2026-03-08", 23), ("2026-11-01", 25)] {
        let d = repo::validate_date(date).unwrap();
        let from =
            chrono::DateTime::parse_from_rfc3339(&reports::boundary_in(d, &zone).unwrap()).unwrap();
        let until = chrono::DateTime::parse_from_rfc3339(
            &reports::boundary_in(d + Duration::days(1), &zone).unwrap(),
        )
        .unwrap();
        assert_eq!((until - from).num_hours(), hours);
    }
}

#[test]
fn cross_month_upcoming_and_historical_category_snapshot() {
    with_db(|db| {
        let mut c = db.lock().unwrap();
        let mut i = input("跨月推进");
        i.due_date_local = Some("2024-03-04".into());
        i.category_id = Some("work".into());
        let t = repo::save_task(&mut c, i).unwrap();
        move_events(&c, &t.id, "2024-02-20");
        repo::save_log(
            &c,
            LogInput {
                id: None,
                kind: "progress".into(),
                task_id: Some(t.id.clone()),
                local_date: "2024-02-29".into(),
                content: "二月进展".into(),
                expected_updated_at: None,
            },
        )
        .unwrap();
        let r = reports::generate(&c, "week", "2024-02-29").unwrap();
        assert_eq!(r.upcoming.len(), 1);
        assert_eq!(manual_count(&r, "progress"), 1);
        assert_eq!(r.unfinished.len(), 1);
        repo::set_status(&mut c, &t.id, "completed").unwrap();
        c.execute(
            "UPDATE task_events SET created_at_utc=?1 WHERE task_id=?2 AND event_type='completed'",
            params![
                reports::boundary(repo::validate_date("2024-02-29").unwrap()).unwrap(),
                t.id
            ],
        )
        .unwrap();
        repo::save_category(&c, Some("work".into()), "改名后的工作", "#abcdef").unwrap();
        let r = reports::generate(&c, "month", "2024-02-29").unwrap();
        assert_eq!(completion_count(&r), 1);
        let completion = repo::all_events(&c)
            .unwrap()
            .into_iter()
            .find(|event| event.event_type == "completed")
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_str(&completion.snapshot_json).unwrap();
        assert_eq!(snapshot["category_name_snapshot"], "工作");
    });
}

#[test]
fn complete_business_loop_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let date = chrono::Local::now().date_naive().to_string();
    {
        let db = Database::open(dir.path()).unwrap();
        let mut c = db.lock().unwrap();
        let mut i = input("完成项目代码优化");
        i.due_date_local = Some(date.clone());
        i.due_time_local = Some("18:00".into());
        let t = repo::save_task(&mut c, i).unwrap();
        assert_eq!(crate::db::snapshot(&c, dir.path()).unwrap().tasks.len(), 1);
        repo::set_status(&mut c, &t.id, "completed").unwrap();
        repo::save_log(
            &c,
            LogInput {
                id: None,
                kind: "completed".into(),
                task_id: None,
                local_date: date.clone(),
                content: "今天帮助同事定位并解决了一个程序异常。".into(),
                expected_updated_at: None,
            },
        )
        .unwrap();
        let mut s = Settings::default();
        s.widget_mode = "topmost".into();
        crate::db::save_settings(&c, &s).unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    let c = db.lock().unwrap();
    let r = reports::generate(&c, "day", &date).unwrap();
    assert_eq!(completion_count(&r), 1);
    assert_eq!(manual_count(&r, "completed"), 1);
    assert_eq!(completion_title(&r), "完成项目代码优化");
    assert_eq!(crate::db::settings(&c).unwrap().widget_mode, "topmost");
}

#[test]
fn editing_other_fields_preserves_deadline_from_original_timezone() {
    with_db(|db| {
        let mut c = db.lock().unwrap();
        let mut i = input("跨时区任务");
        i.due_date_local = Some("2026-10-17".into());
        i.due_time_local = Some("18:00".into());
        let t = repo::save_task(&mut c, i).unwrap();
        // Simulate a deadline saved in a zone three hours away, without changing the host timezone.
        let fixed = (chrono::DateTime::parse_from_rfc3339(t.due_at_utc.as_ref().unwrap())
            .unwrap()
            .with_timezone(&chrono::Utc)
            + Duration::hours(3))
        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
        c.execute(
            "UPDATE tasks SET due_at_utc=?1 WHERE id=?2",
            params![fixed, t.id],
        )
        .unwrap();
        let t = repo::task(&c, &t.id).unwrap();
        let mut i = update(&t);
        i.description = "只修改描述".into();
        let t = repo::save_task(&mut c, i).unwrap();
        assert_eq!(t.due_at_utc.as_deref(), Some(fixed.as_str()));
        let mut i = update(&t);
        i.due_time_local = Some("19:00".into());
        let t = repo::save_task(&mut c, i).unwrap();
        assert_ne!(t.due_at_utc.as_deref(), Some(fixed.as_str()));
        assert_eq!(
            repo::all_events(&c)
                .unwrap()
                .iter()
                .filter(|e| e.event_type == "due_changed")
                .count(),
            1
        );
    });
}
