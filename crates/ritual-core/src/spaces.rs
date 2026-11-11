//! Local conversations. A new thread only exists once its first message is saved.
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Space {
    pub id: Uuid,
    pub name: String,
    pub goal: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Thread {
    pub id: Uuid,
    pub space: Option<Uuid>,
    pub title: String,
}

#[derive(Clone, Default)]
pub struct SpaceIndex {
    pub spaces: Vec<Space>,
    pub threads: Vec<Thread>,
}

impl Store {
    pub fn space_index(&self) -> Result<SpaceIndex> {
        let mut spaces = self.0.prepare("SELECT id, name, goal FROM spaces ORDER BY rowid")?;
        let mut threads = self.0.prepare("SELECT id, space_id, title FROM threads ORDER BY rowid DESC")?;
        Ok(SpaceIndex {
            spaces: spaces.query_map([], |r| Ok(Space { id: r.get(0)?, name: r.get(1)?, goal: r.get(2)? }))?
                .collect::<rusqlite::Result<_>>()?,
            threads: threads.query_map([], |r| Ok(Thread { id: r.get(0)?, space: r.get(1)?, title: r.get(2)? }))?
                .collect::<rusqlite::Result<_>>()?,
        })
    }

    pub fn create_space(&self, name: &str, goal: &str) -> Result<Space> {
        ensure!(!name.trim().is_empty(), "Enter a name for this space.");
        let space = Space { id: Uuid::new_v4(), name: name.trim().into(), goal: goal.trim().into() };
        self.0.execute("INSERT INTO spaces(id, name, goal) VALUES (?, ?, ?)", params![space.id, space.name, space.goal])?;
        Ok(space)
    }

    pub fn start_thread(&mut self, space: Option<Uuid>, body: &str) -> Result<Thread> {
        let body = body.trim();
        ensure!(!body.is_empty(), "Write a message first.");
        let title = body.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(70).collect();
        let thread = Thread { id: Uuid::new_v4(), space, title };
        let tx = self.0.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO threads(id, space_id, title) VALUES (?, ?, ?)", params![thread.id, space, thread.title])?;
        tx.execute("INSERT INTO thread_messages(thread_id, body) VALUES (?, ?)", params![thread.id, body])?;
        tx.commit()?;
        Ok(thread)
    }

    pub fn add_message(&self, thread: Uuid, body: &str) -> Result<()> {
        ensure!(!body.trim().is_empty(), "Write a message first.");
        self.0.execute("INSERT INTO thread_messages(thread_id, body) VALUES (?, ?)", params![thread, body.trim()])?;
        Ok(())
    }

    pub fn messages(&self, thread: Uuid) -> Result<Vec<String>> {
        let mut q = self.0.prepare("SELECT body FROM thread_messages WHERE thread_id = ? ORDER BY sequence")?;
        Ok(q.query_map([thread], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?)
    }
}
