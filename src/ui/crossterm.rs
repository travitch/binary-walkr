use crossterm;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    collections,
    io,
    time::{Duration, Instant}
};
use tui;

use crate::summarize;
use crate::ui::app;
use crate::ui::draw;

fn run_app<B: tui::backend::Backend>(terminal: &mut tui::Terminal<B>, mut app: app::App, tick_rate: Duration) -> anyhow::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| draw::draw(f, &mut app))?;

        let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                app.on_key(key);
            }
        }
        if last_tick.elapsed() >= tick_rate {
            // app.on_tick();
            last_tick = Instant::now();
        }
        if app.should_quit {
            return Ok(());
        }
    }
}

pub fn run(tick_rate : Duration,
           elf : &summarize::ElfSummary,
           resolved_deps : &collections::BTreeMap<String, Option<summarize::ElfSummary>>) -> anyhow::Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = tui::backend::CrosstermBackend::new(stdout);
    let mut terminal = tui::Terminal::new(backend)?;

    // create app and run it
    let app = app::App::new("binary-walkr", elf, resolved_deps);
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}
