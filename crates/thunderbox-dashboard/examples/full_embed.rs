//! An example of a program that holds a whole thunderbox dashboard.
//!
//! The program owns the terminal, the event loop, and its own key bindings.
//! thunderbox runs the queries and holds the state of each panel.
//!
//! Note what the program keeps. It makes the async runtime, and it passes a
//! `Handle` to thunderbox. It reads every key first, and it passes to thunderbox
//! only the keys that it does not want. ADR 0015 in `docs/contributor/adr` states this
//! promise.
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

use std::borrow::Cow;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Size};
use ratatui::widgets::{Block, Paragraph};
use thunderbox_core::{Dashboard, now_ms};
use thunderbox_dashboard::{
    Consumed, DashboardRuntime, DashboardView, Key, Variables, row_key, time_key, variable_key,
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

/// The time that the loop waits for an event.
///
/// The loop wakes at least once in this time, so a key reaches the program at
/// once. A wake that finds no change draws no frame.
const WAKE: Duration = Duration::from_millis(100);

/// The facts that decide whether a wake draws a frame.
///
/// The loop reads these facts on each wake, and it draws a frame only when
/// they differ from the facts of the last frame. A frame that shows the same
/// facts as the frame before it shows the same screen, and ratatui would write
/// no cell for it.
///
/// A program that holds a thunderbox dashboard needs one fact for each input
/// of its own frame. This program has one input besides the dashboard, which
/// is its own line of text.
#[derive(Debug, Clone, PartialEq)]
struct Facts<'a> {
    /// The number of new snapshots that the loop has read.
    snapshots: u64,
    /// The size of the terminal on this wake.
    size: Size,
    /// The number of events that the loop has applied.
    events: u64,
    /// The line of text that the program owns. A program that builds its text
    /// at run time holds a `String` here.
    own_message: &'static str,
    /// The variables that the row of the variables shows. A wake that draws no
    /// frame borrows them. The copy of the last frame owns them.
    variables: Cow<'a, Variables>,
}

impl Facts<'_> {
    /// Returns the same facts with a copy of the variables.
    fn into_owned(self) -> Facts<'static> {
        Facts {
            snapshots: self.snapshots,
            size: self.size,
            events: self.events,
            own_message: self.own_message,
            variables: Cow::Owned(self.variables.into_owned()),
        }
    }
}

/// Returns `true` when the wake must draw a frame.
///
/// The wake must draw when no frame exists yet, and when a fact differs from
/// the fact of the last frame.
fn must_draw(last: Option<&Facts<'_>>, now: &Facts<'_>) -> bool {
    last != Some(now)
}

/// Reads the events that wait, and returns them.
///
/// The function asks `poll` whether an event waits, and it passes the time
/// that `poll` may wait. The first call waits for [`WAKE`]. Each later call
/// waits for no time, so the function returns when the queue holds no more
/// events. A burst of keys therefore reaches the loop in one wake.
fn read_burst(
    mut poll: impl FnMut(Duration) -> io::Result<bool>,
    mut read: impl FnMut() -> io::Result<Event>,
) -> io::Result<Vec<Event>> {
    let mut events = Vec::new();
    let mut wait = WAKE;
    while poll(wait)? {
        events.push(read()?);
        wait = Duration::ZERO;
    }
    Ok(events)
}

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
    // The receiver reports each change of the snapshot. A comparison of the
    // `Arc` would miss a change that edits the value in place.
    let mut snapshots = dash.subscribe();
    let mut last: Option<Facts<'static>> = None;
    let mut panel_area = Size::new(0, 0);
    let mut snapshot_count = 0_u64;
    let mut event_count = 0_u64;

    loop {
        {
            let size = terminal.size()?;
            let new_snapshot = snapshots.has_changed().unwrap_or(true);
            // The loop marks the change as seen only when it also reads the
            // snapshot that holds the change.
            let snapshot = if new_snapshot {
                snapshots.borrow_and_update().clone()
            } else {
                snapshots.borrow().clone()
            };
            snapshot_count += u64::from(new_snapshot);
            let facts = Facts {
                snapshots: snapshot_count,
                size,
                events: event_count,
                own_message,
                variables: Cow::Borrowed(dash.variables()),
            };
            if must_draw(last.as_ref(), &facts) {
                // The runtime holds the dashboard, and the program borrows it.
                // The frame needs no copy of the dashboard.
                let dashboard = dash.dashboard();
                // A dashboard that declares no variable draws no row for one.
                let variable_height = u16::from(!facts.variables.is_empty());

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
                        Paragraph::new(facts.own_message)
                            .block(Block::bordered().title("the program")),
                        top,
                    );

                    // thunderbox writes the row of the variables, and the
                    // program decides where that row sits.
                    if variable_height > 0 {
                        frame.render_widget(Paragraph::new(facts.variables.row_text()), row);
                    }

                    // thunderbox draws the dashboard inside the area that the
                    // program gives it.
                    let buf = frame.buffer_mut();
                    view.render(dashboard, &snapshot, middle, buf);
                    panel_area = middle.as_size();

                    // This text is a copy of `KEYS` of
                    // `crates/thunderbox/src/main.rs`, character for character.
                    // This crate cannot depend on the crate that holds that
                    // constant, so no test compares the two. Change both texts
                    // together.
                    frame.render_widget(
                        Paragraph::new(
                            "q quit  tab next panel  m marker  r refresh  \
                             arrows move  -+ width  0 range",
                        ),
                        bottom,
                    );
                })?;
                last = Some(facts.into_owned());
            }
        }

        // The program reports the whole area that thunderbox holds. thunderbox
        // divides that area onto the panels, and each panel then asks its data
        // source for the points of its own width. thunderbox asks for nothing at
        // all for a panel that it draws no picture of. A wake that draws no
        // frame reports the area of the last frame.
        dash.set_panel_area(panel_area);
        dash.set_marker(view.marker());
        // The program drives the refresh from its own event loop.
        dash.tick();

        // The loop waits for the first event, and then it applies each event
        // that waits. A burst of keys therefore costs one frame.
        for event in read_burst(event::poll, event::read)? {
            let Event::Key(key) = event else {
                event_count += 1;
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            event_count += 1;
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
            // thunderbox reads the rest, and it reports what it used. The
            // loop reads the snapshot again for each key, because an earlier
            // key of the same burst can change it.
            let snapshot = dash.snapshot();
            if view.handle_key(translated, dash.dashboard(), &snapshot) == Consumed::No {
                own_message = "thunderbox did not use that key, so the program keeps it.";
            } else {
                own_message = "thunderbox used that key.";
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the facts of a wake with no change.
    fn facts(variables: &Variables) -> Facts<'_> {
        Facts {
            snapshots: 1,
            size: Size::new(80, 24),
            events: 0,
            own_message: "The program owns this line.",
            variables: Cow::Borrowed(variables),
        }
    }

    #[test]
    fn the_first_wake_draws_a_frame() {
        let variables = Variables::default();
        assert!(
            must_draw(None, &facts(&variables)),
            "No frame exists yet, so the loop must draw one."
        );
    }

    #[test]
    fn a_wake_with_no_change_draws_no_frame() {
        let variables = Variables::default();
        let last = facts(&variables).into_owned();
        assert!(
            !must_draw(Some(&last), &facts(&variables)),
            "A wake with the same facts must draw no frame."
        );
    }

    #[test]
    fn each_fact_alone_draws_a_frame() {
        let variables = Variables::default();
        let last = facts(&variables).into_owned();

        let mut snapshot = facts(&variables);
        snapshot.snapshots += 1;
        assert!(
            must_draw(Some(&last), &snapshot),
            "A new snapshot must draw a frame."
        );

        let mut size = facts(&variables);
        size.size = Size::new(81, 24);
        assert!(
            must_draw(Some(&last), &size),
            "A new size must draw a frame."
        );

        let mut event = facts(&variables);
        event.events += 1;
        assert!(
            must_draw(Some(&last), &event),
            "An applied event must draw a frame."
        );

        let mut text = facts(&variables);
        text.own_message = "The program changed the time range.";
        assert!(
            must_draw(Some(&last), &text),
            "A new line of text of the program must draw a frame."
        );
    }

    #[test]
    fn the_loop_waits_for_the_first_event_and_reads_the_rest_with_no_wait() {
        let mut waits = Vec::new();
        let queued = std::cell::Cell::new(3_u32);
        let events = read_burst(
            |wait| {
                waits.push(wait);
                Ok(queued.get() > 0)
            },
            || {
                queued.set(queued.get() - 1);
                Ok(Event::Key(KeyCode::Tab.into()))
            },
        )
        .expect("The queue must not fail.");
        assert_eq!(
            events.len(),
            3,
            "One wake must read the whole burst, and not one key."
        );
        assert_eq!(
            waits,
            vec![WAKE, Duration::ZERO, Duration::ZERO, Duration::ZERO],
            "Only the first call waits."
        );
    }

    #[test]
    fn a_change_of_the_variables_draws_a_frame() {
        let declared = [thunderbox_core::Variable {
            name: "job".to_string(),
            label: None,
            options: vec!["web".into(), "db".into()],
            query: None,
            current: None,
        }];
        let before = Variables::new(&declared);
        let after = variable_key(Key::Char(']'), &before)
            .expect("The key must change the selection of the variable.");
        let last = facts(&before).into_owned();
        assert!(
            must_draw(Some(&last), &facts(&after)),
            "A new value of a variable must draw a frame."
        );
    }
}
