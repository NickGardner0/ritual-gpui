CREATE TABLE habits (
    id BLOB PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    category TEXT NOT NULL CHECK (length(trim(category)) > 0),
    unit_type TEXT NOT NULL CHECK (length(trim(unit_type)) > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE habit_logs (
    id BLOB PRIMARY KEY NOT NULL,
    habit_id BLOB NOT NULL REFERENCES habits(id),
    client_event_id BLOB NOT NULL,
    amount REAL NOT NULL CHECK (amount > 0 AND amount <= 1.7976931348623157e308),
    date TEXT NOT NULL,
    completed_at TEXT NOT NULL,
    utc_offset_seconds INTEGER NOT NULL CHECK (abs(utc_offset_seconds) < 86400),
    source TEXT NOT NULL DEFAULT 'manual',
    status TEXT NOT NULL DEFAULT 'completed',
    UNIQUE (habit_id, client_event_id)
);

CREATE INDEX habit_log_history ON habit_logs(habit_id, completed_at DESC, id DESC);
PRAGMA user_version = 1;
