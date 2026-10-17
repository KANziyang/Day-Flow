use crate::{models::*, repository};
use anyhow::{bail, Result};
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct Report {
    pub kind: String,
    pub start: String,
    pub end: String,
    pub next_start: String,
    pub next_end: String,
    pub work_records: Vec<WorkRecord>,
    pub unfinished: Vec<Task>,
    pub upcoming: Vec<Task>,
}
pub fn period(kind: &str, date: NaiveDate) -> Result<(NaiveDate, NaiveDate)> {
    let start = match kind {
        "day" => date,
        "week" => date - Duration::days(date.weekday().num_days_from_monday() as i64),
        "month" => date.with_day(1).unwrap(),
        _ => bail!("报表类型无效"),
    };
    let end = match kind {
        "day" => start.checked_add_signed(Duration::days(1)),
        "week" => start.checked_add_signed(Duration::days(7)),
        _ => {
            if start.month() == 12 {
                NaiveDate::from_ymd_opt(start.year() + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(start.year(), start.month() + 1, 1)
            }
        }
    }
    .ok_or_else(|| anyhow::anyhow!("日期超出范围"))?;
    Ok((start, end))
}
pub fn boundary(date: NaiveDate) -> Result<String> {
    boundary_in(date, &Local)
}
pub fn boundary_in<T: TimeZone>(date: NaiveDate, zone: &T) -> Result<String> {
    // Calendar boundaries are calculated separately; never assume every day has 24 hours.
    let instant = zone
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
        .earliest()
        .ok_or_else(|| anyhow::anyhow!("该日期在本地时区不存在"))?;
    Ok(instant
        .with_timezone(&Utc)
        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true))
}
pub fn generate(c: &Connection, kind: &str, date: &str) -> Result<Report> {
    let (start, end) = period(kind, repository::validate_date(date)?)?;
    let from = boundary(start)?;
    let until = boundary(end)?;
    generate_between(c, kind, start, end, &from, &until)
}
pub fn generate_between(
    c: &Connection,
    kind: &str,
    start: NaiveDate,
    end: NaiveDate,
    from: &str,
    until: &str,
) -> Result<Report> {
    let (next_start, next_end) = period(kind, end)?;
    let events = repository::all_events(c)?;
    let mut states: HashMap<String, Task> = HashMap::new();
    let mut completed: HashMap<String, TaskEvent> = HashMap::new();
    for e in &events {
        if e.created_at_utc.as_str() < until {
            let state: Task = serde_json::from_str(&e.snapshot_json)?;
            states.insert(e.task_id.clone(), state);
        }
        if e.created_at_utc.as_str() >= from
            && e.created_at_utc.as_str() < until
            && e.event_type == "completed"
        {
            completed.insert(e.task_id.clone(), e.clone());
        }
    }
    let logs = repository::logs(c)?
        .into_iter()
        .filter(|l| l.local_date >= start.to_string() && l.local_date < end.to_string())
        .collect::<Vec<_>>();
    let mut work_records = completed
        .values()
        .map(|event| repository::completion_record(event, &events))
        .collect::<Result<Vec<_>>>()?;
    work_records.extend(logs.into_iter().map(|log| WorkRecord::Manual { log }));
    repository::sort_work_records(&mut work_records);
    let mut unfinished: Vec<_> = states
        .into_values()
        .filter(|t| {
            t.deleted_at_utc.is_none() && ["todo", "in_progress"].contains(&t.status.as_str())
        })
        .collect();
    unfinished.sort_by_key(|t| {
        (
            t.due_date_local.clone().unwrap_or("9999-12-31".into()),
            t.due_time_local.clone().unwrap_or("23:59".into()),
            match t.priority.as_str() {
                "high" => 0,
                "medium" => 1,
                _ => 2,
            },
            t.created_at_utc.clone(),
        )
    });
    let upcoming = unfinished
        .iter()
        .filter(|t| {
            t.due_date_local
                .as_ref()
                .is_some_and(|d| d >= &next_start.to_string() && d < &next_end.to_string())
        })
        .cloned()
        .collect();
    Ok(Report {
        kind: kind.into(),
        start: start.to_string(),
        end: (end - Duration::days(1)).to_string(),
        next_start: next_start.to_string(),
        next_end: (next_end - Duration::days(1)).to_string(),
        work_records,
        unfinished,
        upcoming,
    })
}
