use super::*;

#[test]
fn water_survives_reopen_and_retries_are_idempotent() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("ritual.sqlite3");
    let mut store = Store::open(&path)?;
    let id = Uuid::new_v4();
    let habit = store.create_habit(id, " Water ", "Health", " glasses ")?;
    assert_eq!(habit, store.create_habit(id, "Water", "Health", "glasses")?);
    assert!(store.create_habit(id, "Coffee", "Health", "cups").is_err());
    let mut command = ManualLog::now(id, 2.);
    command.at = DateTime::parse_from_rfc3339("2026-11-10T23:52:00-05:00")?;
    let saved = store.append(&command)?;
    drop(store);

    let mut store = Store::open(&path)?;
    assert_eq!(store.habits()?, [habit]);
    assert_eq!(store.history(id)?.as_slice(), std::slice::from_ref(&saved));
    assert_eq!(saved.date.to_string(), "2026-11-10");
    assert_eq!(saved.completed_at.to_rfc3339(), "2026-11-11T04:52:00+00:00");
    assert_eq!(saved.display_time(), "Nov 10, 2026 · 11:52 PM");
    assert_eq!((saved.amount, saved.source.as_str(), saved.status.as_str()), (2., "manual", "completed"));
    assert_eq!(store.append(&command)?, saved);
    command.amount = 1.;
    assert!(store.append(&command).is_err());
    command.event_id = Uuid::new_v4();
    command.at = DateTime::parse_from_rfc3339("2026-11-11T08:00:00-05:00")?;
    let later = store.append(&command)?;
    assert_eq!(store.history(id)?, [later, saved]);
    Ok(())
}

#[test]
fn failed_writes_retain_the_submission_for_retry() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut store = Store::open(&dir.path().join("ritual.sqlite3"))?;
    let id = Uuid::new_v4();
    for (name, category, unit) in [(" ", "Health", "glasses"), ("Water", "", "glasses"), ("Water", "Health", " ")] {
        assert!(store.create_habit(id, name, category, unit).is_err());
    }
    let log = ManualLog::now(id, 2.);
    assert!(store.append(&log).is_err());
    store.create_habit(id, "Water", "Health", "glasses")?;
    for amount in [-2., 0., f64::NAN, f64::INFINITY] {
        assert!(store.append(&ManualLog::now(id, amount)).is_err());
    }
    store.0.pragma_update(None, "query_only", true)?;
    assert!(store.append(&log).is_err());
    assert!(store.history(id)?.is_empty());
    store.0.pragma_update(None, "query_only", false)?;
    let saved = store.append(&log)?;
    assert_eq!(store.append(&log)?, saved);
    assert_eq!(store.history(id)?, [saved]);
    Ok(())
}

#[test]
fn home_index_keeps_empty_habits_and_summarizes_each_unit_after_reopen() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("ritual.sqlite3");
    let mut db = Store::open(&path)?;
    let water = db.create_habit(Uuid::new_v4(), "Water", "Health", "glasses")?;
    let heart = db.create_habit(Uuid::new_v4(), "Heart rate", "Health", "BPM")?;
    let read = db.create_habit(Uuid::new_v4(), "Daily Reading", "Education", "Pages")?;
    let first = ManualLog::now(water.id, 2.);
    db.append(&first)?;
    db.append(&first)?;
    db.append(&ManualLog::now(water.id, 1.5))?;
    db.append(&ManualLog::now(heart.id, 70.))?;
    db.append(&ManualLog::now(heart.id, 80.))?;
    drop(db);
    let rows = Store::open(&path)?.index()?;
    let row = |id| rows.iter().find(|r| r.habit.id == id).unwrap();
    assert_eq!((row(water.id).count, row(water.id).value()), (2, 3.5));
    assert_eq!(row(heart.id).value(), 75.);
    assert_eq!((row(read.id).count, row(read.id).value()), (0, 0.));
    Ok(())
}

#[test]
fn upgrading_a_water_library_preserves_logs_and_both_kinds_of_thread() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("ritual.sqlite3");
    let old = Connection::open(&path)?;
    old.execute_batch(include_str!("001_habits.sql"))?;
    let mut old = Store(old);
    let water = old.create_habit(Uuid::new_v4(), "Water", "Health", "glasses")?;
    let entry = old.append(&ManualLog::now(water.id, 2.))?;
    drop(old);

    let mut db = Store::open(&path)?;
    let space = db.create_space(" Sleep ", "Review evenings")?;
    let inside = db.start_thread(Some(space.id), "Evening walk")?;
    let loose = db.start_thread(None, "Water intake")?;
    db.add_message(inside.id, "After dinner")?;
    assert!(db.start_thread(Some(Uuid::new_v4()), "Orphan").is_err());
    assert!(db.start_thread(None, "   ").is_err());
    drop(db);

    let db = Store::open(&path)?;
    let index = db.space_index()?;
    assert_eq!(index.spaces, [space]);
    assert_eq!(index.threads, [loose.clone(), inside.clone()]);
    assert_eq!(db.messages(inside.id)?, ["Evening walk", "After dinner"]);
    assert_eq!(db.messages(loose.id)?, ["Water intake"]);
    assert_eq!(db.all_logs()?, [("Water".into(), "glasses".into(), entry)]);
    Ok(())
}

#[test]
fn index_range_uses_local_days_and_deletion_preserves_other_habits() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("track.sqlite3");
    let mut store = Store::open(&path)?;
    let water = store.create_habit(Uuid::new_v4(), "Water", "Health", "glasses")?;
    let pulse = store.create_habit(Uuid::new_v4(), "Pulse", "Health", "BPM")?;
    let mut days = store.daily_totals()?;
    for (id, amount, time) in [
        (water.id, 2., "2026-11-10T23:55:00-05:00"),
        (water.id, 1., "2026-11-11T12:00:00-05:00"),
        (pulse.id, 60., "2026-11-10T08:00:00-05:00"),
        (pulse.id, 66., "2026-11-10T09:00:00-05:00"),
        (pulse.id, 90., "2026-11-11T18:00:00-05:00"),
    ] {
        let entry = store.append(&ManualLog { at: DateTime::parse_from_rfc3339(time)?, ..ManualLog::now(id, amount) })?;
        days.record(&entry);
    }
    let rows = store.index()?;
    let day: NaiveDate = "2026-11-10".parse()?;
    for totals in [days, store.daily_totals()?] {
        let row = |id| rows.iter().find(|h| h.habit.id == id).unwrap();
        assert_eq!(totals.value(row(water.id), &(day..=day)), 2.);
        assert_eq!(totals.value(row(pulse.id), &(day..=day)), 63.);
        assert_eq!(totals.value(row(pulse.id), &(day..=day.succ_opt().unwrap())), 72.);
    }
    store.delete_habit(water.id)?;
    drop(store);
    let store = Store::open(&path)?;
    assert!(store.history(water.id)?.is_empty());
    assert_eq!(store.habits()?, [pulse]);
    assert_eq!(store.all_logs()?.len(), 3);
    Ok(())
}
