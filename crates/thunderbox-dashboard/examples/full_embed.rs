//! An example of a program that holds a whole thunderbox dashboard.
//!
//! The program owns the terminal, the event loop, and its own key bindings.
//! thunderbox runs the queries and holds the state of each panel.
//!
//! Note what the program keeps. It makes the async runtime, and it passes a
//! `Handle` to thunderbox. It reads every key first, and it passes to thunderbox
//! only the keys that it does not want. `docs/ARCHITECTURE.md` section 12
//! states this promise.
//!
//! The program also binds the keys of the time range, the keys of a variable,
//! and the keys of a row panel. thunderbox consumes none of them.
//! `thunderbox_dashboard::time_key`, `thunderbox_dashboard::variable_key` and
//! `thunderbox_dashboard::row_key` hold the rules that map such a key, and the
//! program decides which key reaches each rule.
//!
//! The dashboard of this example declares no variable, so the row of the
//! variables holds nothing and takes no line. A person who adds a
//! `[[variable]]` table to the text below reads that row, and the three keys
//! then work.
//!
//! Run the example with this command:
//!
//!     cargo run -p thunderbox-dashboard --example full_embed

use std::io;
use std::sync::Arc;
use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Size};
use ratatui::widgets::{Block, Paragraph};
use thunderbox_core::{Dashboard, now_ms};
use thunderbox_dashboard::{
    Consumed, DashboardRuntime, DashboardView, Key, row_key, time_key, variable_key,
};
use thunderbox_source::Registry;
use thunderbox_source_testdata::TestData;

const DASHBOARD: &str = r#"
title = "system"
refresh = "1s"

[source.test]
kind = "testdata"

[time]
from = "now-5m"
to = "now"

[[panel]]
title = "cpu"
kind = "timeseries"
unit = "percent"
grid = { x = 0, y = 0, w = 16, h = 8 }
[[panel.target]]
source = "test"
expr = "random_walk"
legend = "core {{series}}"

[[panel]]
title = "cpu now"
kind = "stat"
unit = "percent"
grid = { x = 16, y = 0, w = 8, h = 8 }
[[panel.threshold]]
value = 70.0
color = "warning"
[[panel.target]]
source = "test"
expr = "random_walk"
legend = "core {{series}}"

[[panel]]
title = "memory"
kind = "gauge"
unit = "percent"
min = 0.0
max = 100.0
grid = { x = 0, y = 8, w = 8, h = 8 }
[[panel.target]]
source = "test"
expr = "sine"
legend = "bank {{series}}"

[[panel]]
title = "peak by core"
kind = "bargauge"
unit = "percent"
min = 0.0
max = 100.0
reduce = "max"
grid = { x = 8, y = 8, w = 8, h = 8 }
[[panel.target]]
source = "test"
expr = "random_walk"
legend = "core {{series}}"

[[panel]]
title = "disk, with holes in the data"
kind = "timeseries"
unit = "percent"
grid = { x = 16, y = 8, w = 8, h = 8 }
[[panel.target]]
source = "test"
expr = "gappy"
legend = "disk {{series}}"
"#;

/// The program makes the async runtime. thunderbox never does.
fn main() -> io::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(run());
    ratatui::restore();
    result
}

async fn run() -> io::Result<()> {
    let dashboard = Dashboard::from_toml(DASHBOARD).expect("The dashboard must be correct.");
    let sources = Registry::new().with("test", Arc::new(TestData::new().with_series(2)));

    let mut dash = DashboardRuntime::new(dashboard, sources, tokio::runtime::Handle::current())
        .expect("The runtime must start.");
    let mut view = DashboardView::new();
    let mut terminal = ratatui::init();
    let mut own_message = "The program owns this line.";

    loop {
        let snapshot = dash.snapshot();
        let dashboard = dash.dashboard().clone();
        let mut panel_area = Size::new(0, 0);
        // A dashboard that declares no variable draws no row for one.
        let variables = dash.variables().row_text();
        let variable_height = u16::from(!dash.variables().is_empty());

        terminal.draw(|frame| {
            let [top, row, middle, bottom] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(variable_height),
                Constraint::Min(10),
                Constraint::Length(1),
            ])
            .areas(frame.area());

            // The program draws its own parts.
            frame.render_widget(
                Paragraph::new(own_message).block(Block::bordered().title("the program")),
                top,
            );

            // thunderbox writes the row of the variables, and the program
            // decides where that row sits.
            if variable_height > 0 {
                frame.render_widget(Paragraph::new(variables.as_str()), row);
            }

            // thunderbox draws the dashboard inside the area that the program
            // gives it.
            let buf = frame.buffer_mut();
            view.render(&dashboard, &snapshot, middle, buf);
            panel_area = middle.as_size();

            // This text is a copy of `KEYS` of `crates/thunderbox/src/main.rs`,
            // character for character. This crate cannot depend on the crate
            // that holds that constant, so no test compares the two. Change
            // both texts together.
            frame.render_widget(
                Paragraph::new(
                    "q quit  tab next panel  m marker  r refresh  \
                     arrows move  -+ width  0 range",
                ),
                bottom,
            );
        })?;

        // The program reports the whole area that thunderbox holds. thunderbox
        // divides that area onto the panels, and each panel then asks its data
        // source for the points of its own width. thunderbox asks for nothing at
        // all for a panel that it draws no picture of.
        dash.set_panel_area(panel_area);
        dash.set_marker(view.marker());
        // The program drives the refresh from its own event loop.
        dash.tick();

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            // The program reads each key first, and it keeps the keys that it
            // wants. thunderbox never sees them.
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('r') => {
                    own_message = "The program asked thunderbox to run the queries again.";
                    dash.refresh();
                    continue;
                }
                _ => {}
            }
            let translated = match key.code {
                KeyCode::Tab => Key::Tab,
                KeyCode::BackTab => Key::BackTab,
                KeyCode::Left => Key::Left,
                KeyCode::Right => Key::Right,
                KeyCode::Enter => Key::Enter,
                KeyCode::Char(c) => Key::Char(c),
                _ => Key::Other,
            };
            // The program binds the keys of the time range itself. thunderbox
            // consumes none of them, and `time_key` holds the rule alone. That
            // function asks for no range when the key changes nothing.
            let asked = time_key(
                translated,
                dash.time_range(),
                &dash.dashboard().time,
                now_ms(),
            );
            if let Some(range) = asked {
                own_message = "The program changed the time range.";
                dash.set_time_range(range);
                continue;
            }
            // The program binds the keys of a variable itself, and
            // `variable_key` holds that rule alone.
            if let Some(selection) = variable_key(translated, dash.variables()) {
                own_message = "The program changed the selection of a variable.";
                dash.set_variables(selection);
                continue;
            }
            // The program binds the keys of a row panel itself, and `row_key`
            // holds that rule alone. The function answers only while a person
            // has selected a row panel, so the program keeps the key at every
            // other time.
            if let Some(row) = row_key(translated, view.selected(), dash.dashboard()) {
                own_message = "The program collapsed or expanded a row panel.";
                dash.toggle_row(row);
                continue;
            }
            // thunderbox reads the rest, and it reports what it used.
            if view.handle_key(translated, dash.dashboard(), &snapshot) == Consumed::No {
                own_message = "thunderbox did not use that key, so the program keeps it.";
            } else {
                own_message = "thunderbox used that key.";
            }
        }
    }
}
