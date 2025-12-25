use chrono::Local;
use size::Size;

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
