CREATE TABLE spaces (
    id BLOB PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    goal TEXT NOT NULL
);
CREATE TABLE threads (
    id BLOB PRIMARY KEY NOT NULL,
    space_id BLOB REFERENCES spaces(id),
    title TEXT NOT NULL
);
CREATE INDEX space_threads ON threads(space_id);
CREATE TABLE thread_messages (
    sequence INTEGER PRIMARY KEY,
    thread_id BLOB NOT NULL REFERENCES threads(id),
    body TEXT NOT NULL CHECK (length(trim(body)) > 0)
);
CREATE INDEX thread_history ON thread_messages(thread_id, sequence);
PRAGMA user_version = 2;
