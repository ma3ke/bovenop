use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::Context;
use chrono::Local;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::Backend;
use ratatui::{Frame, Terminal};
use size::Size;
use sysinfo::{ProcessRefreshKind, RefreshKind};

use crate::Config;
use crate::app::entry::{Entry, EntryLayout, EntryState};

mod draw;
mod entry;

pub struct Application {
    config: Config,
    is_running: bool,

    sys: sysinfo::System,
    refreshes: RefreshKind,

    // TODO: Actually, this is maybe a silly data structure, here, since new pid's should only
    // be appended, not inserted in between.
    entries: BTreeMap<u32, Entry>, // BTreeSet?
}

impl Application {
    pub fn new(config: Config) -> Self {
        // Set up the system monitoring.
        let refreshes_kind =
            ProcessRefreshKind::nothing().with_memory().with_cpu().with_disk_usage();
        let refreshes = RefreshKind::nothing().with_processes(refreshes_kind);
        let sys = sysinfo::System::new_with_specifics(refreshes);

        Self { config, is_running: false, sys, refreshes, entries: BTreeMap::new() }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        self.is_running = true;

        let mut terminal = ratatui::init();

        loop {
            if !self.is_running {
                break;
            }

            self.process_frame(&mut terminal)?;
            self.handle_events()?;
        }

        ratatui::restore();

        Ok(())
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }

    fn process_frame<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> anyhow::Result<()> {
        self.sys.refresh_specifics(self.refreshes);

        // TODO: Currently not loving the way we find processes (also want to do it by program
        // arguments, path, etc). Also, the way we determine whether a process is dead is a bit
        // weird in my opinion.
        let processes = self.sys.processes_by_name(self.config.name.as_ref());
        let mut alive = Vec::new();
        for process in processes {
            // Add new information to the entry.
            let pid = process.pid().as_u32();

            // For a new process, we first create a new entry.
            // If we already know this process, return its entry.
            let entry = self.entries.entry(pid).or_insert_with(|| Entry {
                state: EntryState::Alive,
                name: process.name().to_string_lossy().to_string(),
                // TODO: Reconsider, bit weird but it works for what we want to do.
                query: self.config.name.clone(),
                pid: process.pid().as_u32(),
                // TODO: The time stuff is a bit hastily implemented. Sit with it for a second.
                start: Local::now().naive_local() - Duration::from_secs(process.run_time()),
                // These we will fill in very shortly.
                mem: Default::default(),
                cpu: Default::default(),
                read: Default::default(),
                write: Default::default(),
                layout: EntryLayout::Expanded,
            });

            entry.mem.push(Size::from_bytes(process.memory()));
            entry.cpu.push(process.cpu_usage() / 100.0);
            entry.read.push(Size::from_bytes(process.disk_usage().total_read_bytes));
            entry.write.push(Size::from_bytes(process.disk_usage().total_written_bytes));
            alive.push(pid);
        }

        // Mark entries for which no process information is available anymore as dead.
        self.entries.iter_mut().for_each(|(pid, entry)| {
            if !alive.contains(pid) {
                entry.die()
            }
        });

        // FIXME: For now I'm panicking here on fail, as I do not like the need for
        // where B::Error: Error + Sync + Send + 'static
        // in the method signature.
        terminal.draw(|frame| self.draw(frame)).expect("failed to draw frame");

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let entry_heights = self.entries.values().map(|e| 1 + e.layout.chart_height());

        let n_visible_entries = {
            let mut n = 0;
            let mut total_height = 0;
            for h in entry_heights.clone() {
                total_height += h;
                if total_height > frame.area().height {
                    break;
                }
                n += 1;
            }
            n
        };
        let vertical =
            Layout::vertical(entry_heights.take(n_visible_entries).map(|h| Constraint::Length(h)));
        let rows = vertical.split(frame.area());
        for (&row, entry) in rows.into_iter().zip(self.entries.values()) {
            frame.render_widget(entry, row);
        }
    }

    fn handle_events(&mut self) -> anyhow::Result<()> {
        // TODO: I hate this so much. The input handling is very poor here.
        // Also: known issue, when you just press a bunch of buttons, the updates will
        // happen more frequently. That is a problem for the CPU sampling, actually.
        // Ultimately, I want a thread that does all of the system monitoring for me.
        if event::poll(Duration::from_millis(200)).context("failed to poll event")? {
            match event::read().context("failed to read event")? {
                Event::Key(ke)
                    if ke.code == KeyCode::Char('q')
                        || (ke.code == KeyCode::Char('c')
                            && ke.modifiers.contains(KeyModifiers::CONTROL)) =>
                {
                    self.stop();
                }
                Event::Key(ke) if ke.code == KeyCode::Char('r') => {
                    self.entries.clear();
                }
                Event::Key(ke) if ke.code == KeyCode::Char('E') => {
                    self.entries.values_mut().for_each(|e| e.layout = EntryLayout::Expanded);
                }
                Event::Key(ke) if ke.code == KeyCode::Char('C') => {
                    self.entries.values_mut().for_each(|e| e.layout = EntryLayout::Condensed);
                }
                _ => {}
            }
        }

        Ok(())
    }
}
