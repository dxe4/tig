use anyhow::Result;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io;

mod app;
mod cli;
mod git;
mod search;
mod ui;

use app::App;
use cli::parse_arg;

fn main() -> Result<()> {
    let initial = parse_arg(std::env::args().skip(1));
    let mut app = App::new(initial)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let mut last_tick = std::time::Instant::now();
    let tick_rate = std::time::Duration::from_millis(50);

    loop {
        terminal.draw(|f| ui::draw(f, app))?;
        app.check_message_timeout();

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press && app.handle_key(key)? => break,
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = std::time::Instant::now();
        }
    }
    Ok(())
}
