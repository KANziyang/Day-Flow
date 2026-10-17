use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub category_id: Option<String>,
    pub due_date_local: Option<String>,
    pub due_time_local: Option<String>,
    pub due_at_utc: Option<String>,
    pub created_at_utc: String,
    pub updated_at_utc: String,
    pub completed_at_utc: Option<String>,
    pub deleted_at_utc: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInput {
    pub id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub status: String,
    pub priority: String,
    pub category_id: Option<String>,
    pub due_date_local: Option<String>,
    pub due_time_local: Option<String>,
    pub expected_updated_at: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at_utc: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkLog {
    pub id: String,
    pub task_id: Option<String>,
    pub kind: String,
    pub local_date: String,
    pub content: String,
    pub created_at_utc: String,
    pub updated_at_utc: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogInput {
    pub id: Option<String>,
    pub task_id: Option<String>,
    pub kind: String,
    pub local_date: String,
    pub content: String,
    pub expected_updated_at: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum WorkRecord {
    TaskCompletion {
        id: String,
        event_id: String,
        task_id: String,
        local_date: String,
        content: String,
        created_at_utc: String,
        reopened_at_utc: Option<String>,
    },
    Manual {
        #[serde(flatten)]
        log: WorkLog,
    },
}
impl WorkRecord {
    pub fn id(&self) -> &str {
        match self {
            Self::TaskCompletion { id, .. } => id,
            Self::Manual { log } => &log.id,
        }
    }
    pub fn local_date(&self) -> &str {
        match self {
            Self::TaskCompletion { local_date, .. } => local_date,
            Self::Manual { log } => &log.local_date,
        }
    }
    pub fn created_at_utc(&self) -> &str {
        match self {
            Self::TaskCompletion { created_at_utc, .. } => created_at_utc,
            Self::Manual { log } => &log.created_at_utc,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: String,
    pub task_id: String,
    pub event_type: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub task_title_snapshot: String,
    pub snapshot_json: String,
    pub created_at_utc: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub widget_count: usize,
    pub widget_opacity: f64,
    pub remember_position: bool,
    pub show_completed: bool,
    pub theme: String,
    pub close_to_tray: bool,
    pub completion_animation: bool,
    pub default_priority: String,
    pub notifications: bool,
    pub quiet_start: String,
    pub quiet_end: String,
    pub date_reminder_time: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            widget_count: 5,
            widget_opacity: 1.0,
            remember_position: true,
            show_completed: true,
            theme: "system".into(),
            close_to_tray: true,
            completion_animation: true,
            default_priority: "medium".into(),
            notifications: false,
            quiet_start: "22:00".into(),
            quiet_end: "08:00".into(),
            date_reminder_time: "09:00".into(),
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub tasks: Vec<Task>,
    pub categories: Vec<Category>,
    pub work_records: Vec<WorkRecord>,
    pub settings: Settings,
    pub widget_presentation: crate::windows::Presentation,
    pub data_dir: String,
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
