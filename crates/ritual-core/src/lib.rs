//! Local habit definitions and manual entries for Ritual.
use anyhow::{Context, Result, bail, ensure};
use chrono::{DateTime, FixedOffset, Local, NaiveDate, Utc};
use rusqlite::{Connection, Row, TransactionBehavior, params};
use std::path::Path;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub struct Habit {
    pub id: Uuid,
    pub name: String,
    pub category: String,
    pub unit: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Habit {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self { id: row.get(0)?, name: row.get(1)?, category: row.get(2)?,
            unit: row.get(3)?, created_at: row.get(4)?, updated_at: row.get(5)? })
    }
}

/// One Home row. Rate metrics show the mean of the recorded measurements.
#[derive(Clone, Debug)]
pub struct TrackedHabit {
    pub habit: Habit,
    pub total: f64,
    pub count: usize,
}

impl TrackedHabit {
    pub fn average_metric(&self) -> bool {
        matches!(self.habit.unit.to_ascii_lowercase().as_str(), "bpm" | "percentage" | "%" | "percent" | "hrv" | "mmhg" | "temperature" | "blood sugar" | "blood pressure" | "sleep score" | "recovery score")
    }
    pub fn value(&self) -> f64 {
        if self.average_metric() && self.count > 0 { self.total / self.count as f64 } else { self.total }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: Uuid,
    pub habit_id: Uuid,
    pub event_id: Uuid,
    pub amount: f64,
    pub date: NaiveDate,
    pub completed_at: DateTime<Utc>,
    pub offset_seconds: i32,
    pub source: String,
    pub status: String,
}

impl Entry {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self { id: row.get(0)?, habit_id: row.get(1)?, event_id: row.get(2)?,
            amount: row.get(3)?, date: row.get(4)?, completed_at: row.get(5)?,
            offset_seconds: row.get(6)?, source: row.get(7)?, status: row.get(8)? })
    }

    pub fn display_time(&self) -> String {
        match FixedOffset::east_opt(self.offset_seconds) {
            Some(offset) => self.completed_at.with_timezone(&offset).format("%b %-d, %Y · %-I:%M %p").to_string(),
            None => self.completed_at.format("%b %-d, %Y · %H:%M UTC").to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ManualLog {
    pub habit_id: Uuid,
    pub event_id: Uuid,
    pub amount: f64,
    pub at: DateTime<FixedOffset>,
}

impl ManualLog {
    pub fn now(habit_id: Uuid, amount: f64) -> Self {
        Self { habit_id, event_id: Uuid::new_v4(), amount, at: Local::now().fixed_offset() }
    }
}

const ENTRY_COLUMNS: &str = "id, habit_id, client_event_id, amount, date, completed_at, utc_offset_seconds, source, status";

pub struct Store(Connection);

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).with_context(|| format!("Open {}", parent.display()))?;
        }
        let mut db = Connection::open(path).with_context(|| format!("Open {}", path.display()))?;
        db.pragma_update(None, "foreign_keys", true)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        match version {
            0 => tx.execute_batch(include_str!("001_habits.sql"))?,
            1 => (),
            _ => bail!("Unsupported Ritual database version {version}"),
        }
        tx.commit()?;
        Ok(Self(db))
    }

    pub fn create_habit(&mut self, id: Uuid, name: &str, category: &str, unit: &str) -> Result<Habit> {
        let (name, category, unit) = (name.trim(), category.trim(), unit.trim());
        ensure!(!name.is_empty(), "Enter a habit name.");
        ensure!(!category.is_empty(), "Enter a category.");
        ensure!(!unit.is_empty(), "Enter a unit.");
        let tx = self.0.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO habits (id, name, category, unit_type, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?5) ON CONFLICT(id) DO NOTHING",
            params![id, name, category, unit, Utc::now()])?;
        let habit = tx.query_row("SELECT id, name, category, unit_type, created_at, updated_at
            FROM habits WHERE id = ?", [id], Habit::read)?;
        ensure!(habit.name == name && habit.category == category && habit.unit == unit,
            "This submission already created a different habit.");
        tx.commit()?;
        Ok(habit)
    }

    pub fn habits(&self) -> Result<Vec<Habit>> {
        let mut query = self.0.prepare("SELECT id, name, category, unit_type, created_at, updated_at
            FROM habits ORDER BY name COLLATE NOCASE, id")?;
        Ok(query.query_map([], Habit::read)?.collect::<rusqlite::Result<_>>()?)
    }

    pub fn index(&self) -> Result<Vec<TrackedHabit>> {
        let mut q = self.0.prepare("SELECT h.id, name, category, unit_type, created_at, updated_at,
            COALESCE(SUM(l.amount), 0.0), COUNT(l.id) FROM habits h
            LEFT JOIN habit_logs l ON l.habit_id = h.id AND l.status = 'completed'
            GROUP BY h.id ORDER BY h.created_at, h.id")?;
        Ok(q.query_map([], |r| Ok(TrackedHabit { habit: Habit::read(r)?, total: r.get(6)?, count: r.get(7)? }))?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn append(&mut self, log: &ManualLog) -> Result<Entry> {
        ensure!(log.amount.is_finite() && log.amount > 0., "Enter an amount greater than zero.");
        let completed_at = log.at.to_utc();
        let offset = log.at.offset().local_minus_utc();
        let tx = self.0.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO habit_logs (id, habit_id, client_event_id, amount, date, completed_at, utc_offset_seconds)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(habit_id, client_event_id) DO NOTHING",
            params![Uuid::new_v4(), log.habit_id, log.event_id, log.amount, log.at.date_naive(), completed_at, offset])?;
        let entry = tx.query_row(&format!("SELECT {ENTRY_COLUMNS} FROM habit_logs WHERE habit_id = ? AND client_event_id = ?"),
            params![log.habit_id, log.event_id], Entry::read)?;
        ensure!(entry.amount == log.amount && entry.completed_at == completed_at && entry.offset_seconds == offset,
            "This submission already saved a different log.");
        tx.commit()?;
        Ok(entry)
    }

    pub fn history(&self, habit_id: Uuid) -> Result<Vec<Entry>> {
        let mut query = self.0.prepare(&format!("SELECT {ENTRY_COLUMNS} FROM habit_logs
            WHERE habit_id = ? ORDER BY completed_at DESC, id DESC"))?;
        Ok(query.query_map([habit_id], Entry::read)?.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests;
