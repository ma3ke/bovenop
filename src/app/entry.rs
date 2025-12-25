use std::time::Duration;

use chrono::Local;
use size::Size;
use sysinfo::Process;

pub struct Entry {
    pub state: EntryState,

    pub name: String,
    pub pid: u32,
    pub start: chrono::NaiveDateTime,
    pub query: String,

    pub mem: Vec<Size>,
    pub cpu: Vec<f32>,
    pub read: Vec<Size>,
    pub write: Vec<Size>,
    pub layout: EntryLayout,
}

impl Entry {
    pub fn new(process: &Process, query: String) -> Self {
        Self {
            state: EntryState::Alive,
            name: process.name().to_string_lossy().to_string(),
            // TODO: Reconsider, bit weird but it works for what we want to do.
            query,
            pid: process.pid().as_u32(),
            // TODO: The time stuff is a bit hastily implemented. Sit with it for a second.
            start: Local::now().naive_local() - Duration::from_secs(process.run_time()),
            mem: Default::default(),
            cpu: Default::default(),
            read: Default::default(),
            write: Default::default(),
            layout: EntryLayout::Expanded,
        }
    }

    pub fn name_match(&self) -> [&str; 3] {
        let (before, after) = self.name.split_once(&self.query).unwrap();
        [before, &self.query, after]
    }

    pub fn die(&mut self) {
        match self.state {
            EntryState::Alive => {
                self.state = EntryState::Dead(Local::now().naive_local());
                self.layout = EntryLayout::Condensed
            }
            EntryState::Dead(_) => {}
        }
    }

    pub fn is_dead(&self) -> bool {
        match self.state {
            EntryState::Alive => false,
            EntryState::Dead(_) => true,
        }
    }
}

pub enum EntryState {
    Alive,
    Dead(chrono::NaiveDateTime),
}

#[derive(Default)]
pub enum EntryLayout {
    #[default]
    Expanded,
    Condensed,
}

impl EntryLayout {
    pub fn chart_height(&self) -> u16 {
        match self {
            EntryLayout::Expanded => 3,
            EntryLayout::Condensed => 1,
        }
    }
}
