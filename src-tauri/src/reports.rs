use crate::{models::*, repository};
use anyhow::{bail, Result};
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

#[derive(Serialize)]
pub struct Report {
    pub kind: String,
    pub start: String,
    pub end: String,
    pub created: usize,
    pub completed: usize,
    pub completion_events: usize,
    pub in_progress: usize,
    pub completions: Vec<TaskEvent>,
    pub logs: Vec<WorkLog>,
    pub unfinished: Vec<Task>,
    pub upcoming: Vec<Task>,
    pub daily: Vec<(String, usize)>,
    pub weekly: Vec<(String, usize)>,
    pub categories: Vec<(String, usize)>,
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
    let events = repository::all_events(c)?;
    let mut states: HashMap<String, Task> = HashMap::new();
    let mut created = 0;
    let mut completion_events = 0;
    let mut completed: HashMap<String, TaskEvent> = HashMap::new();
    let mut daily: BTreeMap<String, std::collections::HashSet<String>> = BTreeMap::new();
    for e in &events {
        if e.created_at_utc.as_str() < until {
            let state: Task = serde_json::from_str(&e.snapshot_json)?;
            states.insert(e.task_id.clone(), state);
        }
        if e.created_at_utc.as_str() >= from && e.created_at_utc.as_str() < until {
            if e.event_type == "created" {
                created += 1;
            }
            if e.event_type == "completed" {
                completion_events += 1;
                let local = chrono::DateTime::parse_from_rfc3339(&e.created_at_utc)?
                    .with_timezone(&Local)
                    .date_naive();
                daily
                    .entry(local.to_string())
                    .or_default()
                    .insert(e.task_id.clone());
                completed.insert(e.task_id.clone(), e.clone());
            }
        }
    }
    let mut completions: Vec<_> = completed.into_values().collect();
    completions.sort_by(|a, b| a.created_at_utc.cmp(&b.created_at_utc));
    let logs = repository::logs(c)?
        .into_iter()
        .filter(|l| l.local_date >= start.to_string() && l.local_date < end.to_string())
        .collect::<Vec<_>>();
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
            t.due_date_local.as_ref().is_some_and(|d| {
                d >= &end.to_string() && d < &(end + Duration::days(7)).to_string()
            })
        })
        .cloned()
        .collect();
    let in_progress = unfinished
        .iter()
        .filter(|t| t.status == "in_progress")
        .count();
    let mut category_counts: BTreeMap<String, usize> = BTreeMap::new();
    for e in &completions {
        let snapshot: serde_json::Value = serde_json::from_str(&e.snapshot_json)?;
        let name = snapshot["category_name_snapshot"]
            .as_str()
            .unwrap_or("未分类")
            .to_owned();
        *category_counts.entry(name).or_default() += 1;
    }
    let mut weekly: BTreeMap<String, usize> = BTreeMap::new();
    let mut days = Vec::new();
    let mut d = start;
    while d < end {
        let count = daily.get(&d.to_string()).map_or(0, |x| x.len());
        days.push((d.to_string(), count));
        let monday = d - Duration::days(d.weekday().num_days_from_monday() as i64);
        *weekly.entry(monday.to_string()).or_default() += count;
        d += Duration::days(1);
    }
    Ok(Report {
        kind: kind.into(),
        start: start.to_string(),
        end: (end - Duration::days(1)).to_string(),
        created,
        completed: completions.len(),
        completion_events,
        in_progress,
        completions,
        logs,
        unfinished,
        upcoming,
        daily: days,
        weekly: weekly.into_iter().collect(),
        categories: category_counts.into_iter().collect(),
    })
}
