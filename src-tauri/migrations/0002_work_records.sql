CREATE TABLE work_logs_v2 (
 id TEXT PRIMARY KEY, task_id TEXT REFERENCES tasks(id),
 kind TEXT NOT NULL CHECK(kind IN ('completed','progress')), local_date TEXT NOT NULL,
 content TEXT NOT NULL CHECK(length(trim(content)) BETWEEN 1 AND 20000),
 created_at_utc TEXT NOT NULL, updated_at_utc TEXT NOT NULL
);
INSERT INTO work_logs_v2(id,task_id,kind,local_date,content,created_at_utc,updated_at_utc)
 SELECT id,task_id,'progress',local_date,content,created_at_utc,updated_at_utc FROM work_logs;
DROP TABLE work_logs;
ALTER TABLE work_logs_v2 RENAME TO work_logs;
CREATE INDEX logs_date ON work_logs(local_date);
