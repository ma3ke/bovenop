use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::Context;
use chrono::Local;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Widget};
use size::Size;
use sysinfo::{ProcessRefreshKind, RefreshKind};

use crate::Config;
use crate::entry::{Entry, EntryLayout, EntryState};

pub fn run(config: Config) -> anyhow::Result<()> {
    // Set up the system monitoring.
    let refreshes = RefreshKind::nothing()
        .with_processes(ProcessRefreshKind::nothing().with_memory().with_cpu().with_disk_usage());
    let mut sys = sysinfo::System::new_with_specifics(refreshes);

    // TODO: Actually, this is maybe a silly data structure, here, since new pid's should only
    // be appended, not inserted in between.
    let mut entries = BTreeMap::<u32, Entry>::new(); // BTreeSet?

    let mut terminal = ratatui::init();
    loop {
        sys.refresh_specifics(refreshes);
        // TODO: Currently not loving the way we find processes (also want to do it by program
        // arguments, path, etc). Also, the way we determine whether a process is dead is a bit
        // weird in my opinion.
        let processes = sys.processes_by_name(config.name.as_ref());
        let mut alive = Vec::new();
        for process in processes {
            // Add new information to the entry.
            let pid = process.pid().as_u32();

            // For a new process, we first create a new entry.
            // If we already know this process, return its entry.
            let entry = entries.entry(pid).or_insert_with(|| Entry {
                state: EntryState::Alive,
                name: process.name().to_string_lossy().to_string(),
                // TODO: Reconsider, bit weird but it works for what we want to do.
                query: config.name.clone(),
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
        entries.iter_mut().for_each(|(pid, entry)| {
            if !alive.contains(pid) {
                entry.die()
            }
        });

        terminal
            .draw(|frame| {
                let entry_heights = entries.values().map(|e| 1 + e.layout.chart_height());

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
                let vertical = Layout::vertical(
                    entry_heights.take(n_visible_entries).map(|h| Constraint::Length(h)),
                );
                let rows = vertical.split(frame.area());
                for (&row, entry) in rows.into_iter().zip(entries.values()) {
                    frame.render_widget(entry, row);
                }
            })
            .context("failed to draw frame")?;

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
                    break;
                }
                Event::Key(ke) if ke.code == KeyCode::Char('r') => {
                    entries.clear();
                }
                Event::Key(ke) if ke.code == KeyCode::Char('E') => {
                    entries.values_mut().for_each(|e| e.layout = EntryLayout::Expanded);
                }
                Event::Key(ke) if ke.code == KeyCode::Char('C') => {
                    entries.values_mut().for_each(|e| e.layout = EntryLayout::Condensed);
                }
                _ => {}
            }
        }
    }

    ratatui::restore();

    Ok(())
}

mod colors {
    use ratatui::style::Color;

    pub const INFO: Color = Color::from_u32(0x808a9f);
    pub const INFO_NAME: Color = Color::from_u32(0xd29dc0);
    pub const INFO_MATCH: Color = Color::from_u32(0xff5cb0);
    pub const MEM: Color = Color::from_u32(0xe280c1);
    pub const CPU: Color = Color::from_u32(0xbad29f);
    pub const DISK_READ: Color = Color::from_u32(0x8fa7e0);
    pub const DISK_WRITE: Color = Color::from_u32(0xf6ab65);
}

impl Widget for &Entry {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let layout = Layout::vertical([
            Constraint::Length(1),                          // Entry header.
            Constraint::Length(self.layout.chart_height()), // Info, charts.
        ]);

        let entry_layout = Layout::horizontal([
            Constraint::Length(22), // Info.
            Constraint::Fill(1),    // Memory.
            Constraint::Fill(1),    // CPU.
            Constraint::Fill(1),    // Disk I/O.
        ])
        .spacing(1);
        let [info_area, mem_area, cpu_area, disk_area] = entry_layout.areas(area);

        // If a process is dead, we want to dim some of its colors.
        let wilted = if self.is_dead() { Modifier::DIM } else { Modifier::default() };

        // General information about the process.
        {
            let duration = match self.state {
                EntryState::Alive => Local::now().naive_local().signed_duration_since(self.start),
                EntryState::Dead(time_of_death) => time_of_death.signed_duration_since(self.start),
            };
            let start_time = match duration.num_days() {
                ..=0 => self.start.format("%H:%M").to_string(),
                _ => self.start.format("%a %b %d %H:%M").to_string(),
            };
            let duration = {
                let days = duration.num_days();
                let hours = duration.num_hours() % 24;
                let minutes = duration.num_minutes() % 60;
                let seconds = duration.num_seconds() % 60;
                match (days, hours, minutes, seconds) {
                    (0, 0, 0, s) => format!("{s}s"),
                    (0, 0, m, s) => format!("{m}m{s:02}s"),
                    (0, h, m, s) => format!("{h}h{m:02}m{s:02}s"),
                    (d, h, m, s) => format!("{d}d{h:02}h{m:02}m{s:02}s"),
                }
            };

            let [before, matched, after] = self.name_match();
            let name = Line::from(vec![
                Span::raw(before).dim(),
                Span::raw(matched).bold().fg(colors::INFO_MATCH),
                Span::raw(after).dim(),
            ])
            .fg(colors::INFO_NAME);
            let pid = Span::from(self.pid.to_string()).italic().fg(colors::INFO).dim();
            let start_time = Span::from(start_time).fg(colors::INFO).dim();
            let duration = Span::from(duration).fg(colors::INFO);
            let info = match self.layout {
                EntryLayout::Expanded => Paragraph::new(vec![
                    name,
                    Line::from(pid).right_aligned(),
                    Line::from(start_time).right_aligned(),
                    Line::from(duration).right_aligned(),
                ]),
                EntryLayout::Condensed => {
                    let mut top = name;
                    top.push_span(Span::raw(" "));
                    top.push_span(pid);
                    Paragraph::new(vec![
                        top,
                        Line::from(vec![duration, Span::raw(" "), start_time]).right_aligned(),
                    ])
                }
            };
            info.add_modifier(wilted).render(info_area, buf);
        }

        // Memory usage.
        {
            let current = self.mem.last().copied().unwrap_or_default();
            let peak = self.mem.iter().max().copied().unwrap_or_default();
            let header = Line::from(vec![
                Span::from("mem ").fg(colors::MEM),
                Span::from(current.format().to_string()),
                Span::from("  peak ").dim(),
                Span::from(peak.format().to_string()),
            ]);

            let data: Box<[_]> =
                self.mem.iter().enumerate().map(|(x, y)| (x as f64, y.bytes() as f64)).collect();
            let dataset = Dataset::default()
                .data(&data)
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .fg(colors::MEM);
            let chart = Chart::new(vec![dataset])
                .x_axis(Axis::default().bounds([0.0, data.len() as f64 - 1.0]))
                .y_axis(Axis::default().bounds([0.0, peak.bytes() as f64]));

            let [header_area, chart_area] = layout.areas(mem_area);
            header.add_modifier(wilted).render(header_area, buf);
            chart.add_modifier(wilted).render(chart_area, buf);
        }

        // CPU usage.
        {
            let current = self.cpu.last().copied().unwrap_or_default();
            let peak = self.cpu.iter().copied().max_by(f32::total_cmp).unwrap_or_default();
            let header = Line::from(vec![
                Span::from("cpu ").fg(colors::CPU),
                Span::from(format!("{current:>5.2}")),
                Span::from("  peak ").dim(),
                Span::from(format!("{peak:>5.2}")),
            ]);

            let data: Box<[_]> =
                self.cpu.iter().enumerate().map(|(x, y)| (x as f64, *y as f64)).collect();
            let dataset = Dataset::default()
                .data(&data)
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .fg(colors::CPU);
            let chart = Chart::new(vec![dataset])
                .x_axis(Axis::default().bounds([0.0, data.len() as f64 - 1.0]))
                .y_axis(Axis::default().bounds([0.0, peak as f64]));

            let [header_area, chart_area] = layout.areas(cpu_area);
            header.add_modifier(wilted).render(header_area, buf);
            chart.add_modifier(wilted).render(chart_area, buf);
        }

        // Disk I/O.
        {
            let read_total = self.read.last().copied().unwrap_or_default();
            let write_total = self.write.last().copied().unwrap_or_default();
            let header = Line::from(vec![
                Span::from("read ").fg(colors::DISK_READ),
                Span::from(read_total.format().to_string()),
                Span::from("  wrote ").fg(colors::DISK_WRITE),
                Span::from(write_total.format().to_string()),
            ]);

            let read: Box<[_]> =
                self.read.iter().enumerate().map(|(x, y)| (x as f64, y.bytes() as f64)).collect();
            let write: Box<[_]> =
                self.write.iter().enumerate().map(|(x, y)| (x as f64, y.bytes() as f64)).collect();
            let read_dataset = Dataset::default()
                .data(&read)
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .fg(colors::DISK_READ);
            let write_dataset = Dataset::default()
                .data(&write)
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .fg(colors::DISK_WRITE);

            let [header_area, chart_area] = layout.areas(disk_area);
            header.add_modifier(wilted).render(header_area, buf);
            match self.layout {
                EntryLayout::Expanded => {
                    let datasets = vec![read_dataset, write_dataset];
                    let max = Size::max(read_total, write_total).bytes();
                    Chart::new(datasets)
                        .x_axis(Axis::default().bounds([0.0, read.len() as f64 - 1.0]))
                        .y_axis(Axis::default().bounds([0.0, max as f64]))
                        .add_modifier(wilted)
                        .render(chart_area, buf)
                }
                EntryLayout::Condensed => {
                    let read = Chart::new(vec![read_dataset])
                        .x_axis(Axis::default().bounds([0.0, read.len() as f64 - 1.0]))
                        .y_axis(Axis::default().bounds([0.0, read_total.bytes() as f64]));
                    let write = Chart::new(vec![write_dataset])
                        .x_axis(Axis::default().bounds([0.0, write.len() as f64 - 1.0]))
                        .y_axis(Axis::default().bounds([0.0, write_total.bytes() as f64]));
                    let two_charts_layout = Layout::horizontal(Constraint::from_fills([1, 1]));
                    let [read_area, write_area] = two_charts_layout.areas(chart_area);
                    read.add_modifier(wilted).render(read_area, buf);
                    write.add_modifier(wilted).render(write_area, buf);
                }
            }
        }
    }
}
