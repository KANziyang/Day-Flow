CREATE TABLE categories (
 id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE CHECK(length(trim(name)) BETWEEN 1 AND 30),
 color TEXT NOT NULL, created_at_utc TEXT NOT NULL
);
CREATE TABLE tasks (
 id TEXT PRIMARY KEY, title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 500),
 description TEXT NOT NULL DEFAULT '', status TEXT NOT NULL CHECK(status IN ('todo','in_progress','completed','cancelled')),
 priority TEXT NOT NULL CHECK(priority IN ('high','medium','low')), category_id TEXT REFERENCES categories(id) ON DELETE SET NULL,
 due_date_local TEXT, due_time_local TEXT, due_at_utc TEXT,
 created_at_utc TEXT NOT NULL, updated_at_utc TEXT NOT NULL, completed_at_utc TEXT, deleted_at_utc TEXT,
 CHECK(due_time_local IS NULL OR due_date_local IS NOT NULL),
 CHECK((status = 'completed') = (completed_at_utc IS NOT NULL))
);
CREATE TABLE task_events (
 id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id), event_type TEXT NOT NULL,
 old_value TEXT, new_value TEXT, task_title_snapshot TEXT NOT NULL,
 snapshot_json TEXT NOT NULL, created_at_utc TEXT NOT NULL
);
CREATE TABLE work_logs (
 id TEXT PRIMARY KEY, task_id TEXT REFERENCES tasks(id), local_date TEXT NOT NULL,
 content TEXT NOT NULL CHECK(length(trim(content)) BETWEEN 1 AND 20000),
 created_at_utc TEXT NOT NULL, updated_at_utc TEXT NOT NULL
);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at_utc TEXT NOT NULL);
CREATE TABLE notification_deliveries (task_id TEXT NOT NULL REFERENCES tasks(id), due_key TEXT NOT NULL, sent_at_utc TEXT NOT NULL, PRIMARY KEY(task_id,due_key));
CREATE INDEX tasks_active_due ON tasks(deleted_at_utc,status,due_date_local,due_time_local);
CREATE INDEX events_date ON task_events(event_type,created_at_utc);
CREATE INDEX events_task ON task_events(task_id,created_at_utc);
CREATE INDEX logs_date ON work_logs(local_date);
INSERT INTO categories VALUES ('work','工作','#4184ad',strftime('%Y-%m-%dT%H:%M:%fZ','now')),('study','学习','#8580bc',strftime('%Y-%m-%dT%H:%M:%fZ','now')),('life','生活','#579980',strftime('%Y-%m-%dT%H:%M:%fZ','now')),('other','其他','#9b8a70',strftime('%Y-%m-%dT%H:%M:%fZ','now'));
