use clap::Parser;

use crate::app::Application;

mod app;
mod draw;
mod entry;

/// Observe memory, cpu, and disk I/O for processes matching the provided name.
///
/// To clear and reset all entries, press `r`. Use `C` and `E` to collapse and expand all entries,
/// respectively. Exit with `^C` or `q`.
///
/// By Marieke Westendorp, 2025, <ma3ke.cyber@gmail.com>.
#[derive(Parser)]
#[clap(version)]
struct Config {
    /// Name of the program to watch.
    name: String,
}

fn main() -> anyhow::Result<()> {
    let config = Config::parse();
    let mut app = Application::new(config);
    app.start()?;
    Ok(())
}
