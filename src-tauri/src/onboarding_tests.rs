use crate::db::{self, Database};
use rusqlite::Connection;

#[test]
fn new_profile_stays_pending_until_finished_or_skipped() {
    let dir = tempfile::tempdir().unwrap();
    {
        let database = Database::open(dir.path()).unwrap();
        let c = database.lock().unwrap();
        let snapshot = db::snapshot(&c, dir.path()).unwrap();
        assert!(snapshot.onboarding_required);
        assert!(snapshot.tasks.is_empty());
        assert!(snapshot.work_records.is_empty());
    }
    let database = Database::open(dir.path()).unwrap();
    assert!(db::onboarding_required(dir.path()).unwrap());
    let before = serde_json::to_value(db::settings(&database.lock().unwrap()).unwrap()).unwrap();
    db::finish_onboarding(dir.path()).unwrap();
    db::finish_onboarding(dir.path()).unwrap();
    drop(database);
    let database = Database::open(dir.path()).unwrap();
    let c = database.lock().unwrap();
    assert!(!db::snapshot(&c, dir.path()).unwrap().onboarding_required);
    assert_eq!(
        before,
        serde_json::to_value(db::settings(&c).unwrap()).unwrap()
    );
    assert!(db::snapshot(&c, dir.path()).unwrap().tasks.is_empty());
}

#[test]
fn existing_profiles_including_empty_old_databases_do_not_get_a_tutorial() {
    for version in [1, 2, 3] {
        let dir = tempfile::tempdir().unwrap();
        {
            let c = Connection::open(dir.path().join("dayflow.db")).unwrap();
            c.execute_batch(include_str!("../migrations/0001_init.sql"))
                .unwrap();
            if version >= 2 {
                c.execute_batch(include_str!("../migrations/0002_work_records.sql"))
                    .unwrap();
            }
            if version >= 3 {
                c.execute_batch(include_str!("../migrations/0003_remove_widget_mode.sql"))
                    .unwrap();
            }
            c.pragma_update(None, "user_version", version).unwrap();
        }
        let database = Database::open(dir.path()).unwrap();
        assert!(
            !db::snapshot(&database.lock().unwrap(), dir.path())
                .unwrap()
                .onboarding_required
        );
    }
}

#[test]
fn restoring_backups_does_not_change_tutorial_acknowledgment() {
    let dir = tempfile::tempdir().unwrap();
    let database = Database::open(dir.path()).unwrap();
    let pending_backup = database.backup("pending").unwrap();
    db::finish_onboarding(dir.path()).unwrap();
    database.restore(&pending_backup).unwrap();
    assert!(!db::onboarding_required(dir.path()).unwrap());
    let completed_backup = database.backup("completed").unwrap();
    let new_dir = tempfile::tempdir().unwrap();
    let new_database = Database::open(new_dir.path()).unwrap();
    new_database.restore(&completed_backup).unwrap();
    assert!(db::onboarding_required(new_dir.path()).unwrap());
}

#[test]
fn a_failed_acknowledgment_keeps_the_tutorial_pending() {
    let dir = tempfile::tempdir().unwrap();
    let _database = Database::open(dir.path()).unwrap();
    let marker = dir.path().join("onboarding-pending");
    std::fs::remove_file(&marker).unwrap();
    std::fs::create_dir(&marker).unwrap();
    assert!(db::finish_onboarding(dir.path()).is_err());
    assert!(db::onboarding_required(dir.path()).unwrap());
}
