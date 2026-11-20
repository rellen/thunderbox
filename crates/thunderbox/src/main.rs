//! The thunderbox program.
//!
//! This program owns the terminal and the event loop. It drives
//! `thunderbox-dashboard`, which runs the queries. The same runtime serves this
//! program, the SSH server, and another program that holds a thunderbox
//! dashboard. Only the owner of the terminal changes.

use std::path::PathBuf;
use std::time::Duration;

mod sources;

use anyhow::{Context, Result};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Size};
use ratatui::widgets::Paragraph;
use thunderbox_core::{Dashboard, PanelKind, TimeWindow, now_ms};
use thunderbox_dashboard::{
    Consumed, DashboardRuntime, DashboardView, Key, Snapshot, Variables, row_key, time_key,
    variable_key,
};
use thunderbox_widgets::format_window;

/// The dashboard that the program shows when a person names no file.
const DEFAULT_DASHBOARD: &str = include_str!("default.toml");

/// The text that the program writes for the `--help` option.
const HELP: &str = "\
thunderbox is a dashboard platform for the terminal.

The dashboard inside the program reads this machine. It draws the processor,
each core, the memory, the network, and the processes.

Usage:
    thunderbox [FILE]

Arguments:
    FILE    The dashboard to show, as a TOML file. The program shows the
            state of this machine when a person names no file.

Options:
    -h, --help       Write this text and stop.
    -V, --version    Write the version and stop.

Keys:
    q, Esc    Stop the program.
    Tab       Select the next panel.
    Shift+Tab Select the panel before.
    m         Change the glyph that a chart draws a point with.
    r         Run the queries again.
    Left      Move the time range back by half of its width.
    Right     Move the time range forward by half of its width.
    -         Make the time range twice as wide.
    +         Make the time range half as wide.
    0, =      Return to the time range that the dashboard states.
    v         Select the next variable of the dashboard.
    ]         Take the choice that follows, on the variable that you selected.
    [         Take the choice before it, on the variable that you selected.
    Enter     Collapse or expand the row panel that you selected.
    Space     Do what Enter does.
";

/// The key hint that the status row keeps while the window stands still.
///
/// The row drops the whole list of the keys when a sentence takes the room.
/// A person who reads that the window stands still needs one key of that list.
/// That key returns the range of the dashboard. The row therefore writes this
/// one hint in place of the list.
const RANGE_KEY: &str = "0 range";

/// The keys that the status row lists.
///
/// The row writes a short hint for each key, and it writes two spaces between
/// two hints. A reader then reads the list as one part, and the whole list
/// fits a terminal of 80 columns. The `--help` text and `README.md` hold the
/// complete sentence for each key.
///
/// `crates/thunderbox-dashboard/examples/full_embed.rs` holds a copy of this
/// text, character for character. That crate cannot depend on this one, so no
/// constant serves both places. No test compares the two, and no test compares
/// either one against the keys that `DashboardView::handle_key` binds. Change
/// all of these together, and record each scene again that draws this row.
/// `CLAUDE.md` section 6 names those scenes.
const KEYS: &str = "q quit  tab next panel  m marker  r refresh  arrows move  -+ width  0 range";

/// The key hint that the status row adds for a dashboard that holds a row
/// panel.
///
/// The key `Enter` collapses or expands the row panel that a person selected.
/// [`KEYS`] fills most of a terminal of 80 columns, so the row adds this hint
/// only while it fits. A dashboard that holds no row panel never shows it, so
/// the status row of such a dashboard stays as it was.
const ROW_KEY: &str = "enter fold";

/// The keys that the row of the variables lists.
///
/// The row writes this list after the variables, and it writes it only while
/// the list fits. The `--help` text and `README.md` hold the complete sentence
/// for each key.
const VARIABLE_KEYS: &str = "v next variable  [] value";

/// The gap between two sentences of the status row.
const GAP: &str = "    ";

/// The time that a set of queries must wait before the status row reports it.
///
/// A set of queries is the whole group of queries that one refresh sends. The
/// runtime times each set from the moment when it started that set. A data
/// source that answers at once therefore changes no text, and the row stays
/// quiet on a dashboard that refreshes each second.
const SLOW_ANSWER: Duration = Duration::from_secs(1);

/// The age that the numbers must reach before the status row reports it.
const OLD_NUMBERS: Duration = Duration::from_secs(5);

/// What a person asked the program to do.
enum Command {
    /// Show a dashboard. The program reads the file, or it uses the default.
    Run(Option<PathBuf>),
    /// Write a text and stop.
    Write(String),
}

/// Reads the arguments of the command line.
fn parse_arguments(arguments: Vec<String>) -> Command {
    for argument in &arguments {
        match argument.as_str() {
            "-h" | "--help" => return Command::Write(HELP.to_string()),
            "-V" | "--version" => {
                return Command::Write(format!("thunderbox {}\n", env!("CARGO_PKG_VERSION")));
            }
            _ => {}
        }
    }
    Command::Run(arguments.into_iter().next().map(PathBuf::from))
}

fn main() -> Result<()> {
    match parse_arguments(std::env::args().skip(1).collect()) {
        Command::Write(text) => {
            print!("{text}");
            Ok(())
        }
        Command::Run(path) => {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("The program cannot start the async runtime.")?;
            runtime.block_on(run(path))
        }
    }
}

/// Reads the dashboard that a person named, or the default dashboard.
fn load_dashboard(path: Option<PathBuf>) -> Result<Dashboard> {
    match path {
        None => Dashboard::from_toml(DEFAULT_DASHBOARD)
            .context("The dashboard inside the program is not correct."),
        Some(path) => {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("The program cannot read {}.", path.display()))?;
            Dashboard::from_toml(&text)
                .with_context(|| format!("The dashboard in {} is not correct.", path.display()))
        }
    }
}

/// Reads the dashboard, starts the runtime, and then draws.
///
/// The program reads the dashboard before it touches the terminal. A dashboard
/// that holds a fault therefore writes a message and stops. It sends no control
/// sequence to a program that reads the output of this one.
async fn run(path: Option<PathBuf>) -> Result<()> {
    let dashboard = load_dashboard(path)?;
    // The dashboard names each data source that it needs, and the `sources`
    // module makes them. A person can therefore mix real values and test
    // values on one page.
    let sources = sources::build(&dashboard)
        .context("The program cannot make the data sources of the dashboard.")?;

    let mut dash = DashboardRuntime::new(dashboard, sources, tokio::runtime::Handle::current())
        .context("The program cannot start the dashboard.")?;

    // The program owns the terminal from this line to the call that puts it
    // back. Each call to init has one call to restore that answers it.
    let mut terminal = ratatui::init();
    let result = draw_loop(&mut terminal, &mut dash).await;
    ratatui::restore();
    result
}

/// Returns the age of the numbers that a snapshot holds.
///
/// The function returns `None` while no set of queries has answered.
fn age_of(snapshot: &Snapshot) -> Option<Duration> {
    let asked_at = snapshot.updated_at?;
    u64::try_from(now_ms() - asked_at)
        .ok()
        .map(Duration::from_millis)
}

/// Returns the number of cells that the sentences of the status row need.
fn row_width(parts: &[String]) -> usize {
    let gaps = GAP.chars().count() * parts.len().saturating_sub(1);
    parts.iter().map(|part| part.chars().count()).sum::<usize>() + gaps
}

/// Returns the text of the status row.
///
/// The row is one cell high, so every sentence shares one line. The function
/// writes a complete sentence for each fact, and it ends the row with the list
/// of the keys. It drops the last part while the line is too narrow, so the
/// list of the keys gives way first. A person needs that list least.
/// `CLAUDE.md` states the rule for a text that does not fit, in the section
/// `The rule for a short label inside a panel`.
///
/// That rule has one exception. A row that names a window which stands still
/// keeps [`RANGE_KEY`] after it drops the list. That key returns the range of
/// the dashboard.
///
/// The function reports a refresh only when the caller states that the set of
/// queries waits. A data source that answers at once therefore writes nothing,
/// and the row does not change on each refresh.
///
/// The `held` parameter holds the window while that window stands still in the
/// past. The row then names the width of the window and the distance back to
/// it. A person who reads `no data` in every panel learns from that sentence
/// why the panels are empty. A window that ends at the present moment writes
/// no sentence, because nothing is out of the ordinary.
///
/// The `variable` parameter holds the sentence of a variable that has no
/// choice. `Variables::error` writes that sentence. The row of the variables
/// is a row of short labels, so the sentence belongs here.
///
/// The `dropped` parameter holds the number of panels that the renderer drops.
/// thunderbox draws no panel under 3 cells wide or under 3 cells high, and
/// `DashboardView::dropped_panels` counts those panels. The row names that
/// count, because such a panel holds no data and reaches no data source. A
/// person who reads an empty screen learns nothing without that sentence.
fn status_text(
    errors: usize,
    dropped: usize,
    variable: Option<&str>,
    age: Option<Duration>,
    waits: bool,
    held: Option<TimeWindow>,
    width: u16,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if errors == 1 {
        parts.push("1 panel holds an error.".to_string());
    } else if errors > 1 {
        parts.push(format!("{errors} panels hold an error."));
    }
    if dropped == 1 {
        parts.push("The terminal is too small for 1 panel.".to_string());
    } else if dropped > 1 {
        parts.push(format!("The terminal is too small for {dropped} panels."));
    }
    if let Some(sentence) = variable {
        parts.push(sentence.to_string());
    }
    if waits {
        parts.push("The dashboard waits for an answer.".to_string());
    }
    if let Some(window) = held {
        parts.push(format_window(window));
    }
    if let Some(age) = age.filter(|age| *age >= OLD_NUMBERS) {
        parts.push(format!("The numbers are {} seconds old.", age.as_secs()));
    }
    parts.push(KEYS.to_string());
    while parts.len() > 1 && row_width(&parts) > usize::from(width) {
        parts.pop();
    }
    // The row dropped the list of the keys. A person who reads that the window
    // stands still needs the key that returns the range of the dashboard. The
    // row therefore writes that one hint while it fits.
    if held.is_some() && parts.last().is_some_and(|last| last != KEYS) {
        let mut with_key = parts.clone();
        with_key.push(RANGE_KEY.to_string());
        if row_width(&with_key) <= usize::from(width) {
            parts = with_key;
        }
    }
    parts.join(GAP)
}

/// Adds the hint of the key of a row panel to the end of the status row.
///
/// The function adds [`ROW_KEY`] when three conditions hold:
///
/// - the dashboard holds a row panel;
/// - the row still ends with the list of the keys;
/// - the longer row fits the width.
///
/// The hint then reads as one more part of that list.
fn with_row_key(text: String, rows: bool, width: u16) -> String {
    if !rows || !text.ends_with(KEYS) {
        return text;
    }
    let longer = format!("{text}  {ROW_KEY}");
    if longer.chars().count() <= usize::from(width) {
        longer
    } else {
        text
    }
}

/// Returns the text of the row that names each variable.
///
/// The row writes the label of each variable and the value that it holds.
/// `Variables::row_text` holds that rule, so every program that draws a
/// thunderbox dashboard writes the same words.
///
/// The row ends with the list of the keys of a variable, and it writes that
/// list only while the whole row fits the width. `CLAUDE.md` states the rule
/// for a text that does not fit, in the section `The rule for a short label
/// inside a panel`.
///
/// The function returns an empty text for a dashboard that declares no
/// variable. The program then draws no row at all.
fn variable_row_text(variables: &Variables, width: u16) -> String {
    if variables.is_empty() {
        return String::new();
    }
    let row = variables.row_text();
    let with_keys = format!("{row}{GAP}{VARIABLE_KEYS}");
    if with_keys.chars().count() <= usize::from(width) {
        with_keys
    } else {
        row
    }
}

/// Returns the key of thunderbox that answers a key of the terminal.
///
/// The program keeps `q`, `Esc`, `Ctrl+C`, and `r` before it calls this
/// function. It reads the keys of the time range from the result. thunderbox
/// names the keys that it can read, and every other key becomes
/// [`Key::Other`].
fn translate(code: KeyCode) -> Key {
    match code {
        KeyCode::Tab => Key::Tab,
        KeyCode::BackTab => Key::BackTab,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Enter => Key::Enter,
        KeyCode::Char(c) => Key::Char(c),
        _ => Key::Other,
    }
}

/// Draws the dashboard until a person stops the program.
async fn draw_loop(
    terminal: &mut ratatui::DefaultTerminal,
    dash: &mut DashboardRuntime,
) -> Result<()> {
    let mut view = DashboardView::new();

    loop {
        let snapshot = dash.snapshot();
        let dashboard = dash.dashboard().clone();
        let mut panel_area = Size::new(0, 0);
        // A dashboard that declares no variable draws no row for one. The
        // default dashboard of this program is such a dashboard.
        let variable_height = u16::from(!dash.variables().is_empty());
        let variables = dash.variables().clone();
        let variable_error = dash.variables().error();

        // The runtime times each set of queries from the moment when it started
        // that set. The row therefore reports a data source that answers
        // slowly, and it never reports the age of the frames of this program.
        let waits = dash.refresh_age().is_some_and(|age| age >= SLOW_ANSWER);
        let age = age_of(&snapshot);
        // The row reads the window that the panels draw, and not a window of
        // its own. The row and the axis of time then name one window. The row
        // names that window while it ends before the present moment, and it
        // stays quiet at every other time.
        let held = snapshot.window().filter(|window| window.to < window.now);
        let rows = dashboard
            .panels
            .iter()
            .any(|panel| panel.kind == PanelKind::Row);

        terminal.draw(|frame| {
            let [top, body, status] = Layout::vertical([
                Constraint::Length(variable_height),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .areas(frame.area());
            panel_area = body.as_size();
            view.render(&dashboard, &snapshot, body, frame.buffer_mut());

            if variable_height > 0 {
                let row = variable_row_text(&variables, top.width);
                frame.render_widget(Paragraph::new(row), top);
            }

            let text = status_text(
                snapshot.error_count(),
                view.dropped_panels(&dashboard, &snapshot, body),
                variable_error.as_deref(),
                age,
                waits,
                held,
                status.width,
            );
            let text = with_row_key(text, rows, status.width);
            frame.render_widget(Paragraph::new(text), status);
        })?;

        // The program reports the whole area that holds the panels. thunderbox
        // divides that area onto the panels, and each panel then asks its data
        // source for the points of its own width. thunderbox asks for nothing at
        // all for a panel that it draws no picture of.
        dash.set_panel_area(panel_area);
        dash.set_marker(view.marker());
        // The program drives the refresh. thunderbox starts no timer of its own.
        dash.tick();

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                KeyCode::Char('r') => {
                    dash.refresh();
                    continue;
                }
                _ => {}
            }
            let translated = translate(key.code);
            // The program owns each key of the time range. thunderbox consumes
            // none of them, and `thunderbox_dashboard::time_key` holds the rule
            // alone. `docs/ARCHITECTURE.md` section 12 states that rule. That
            // function asks for no range when the key changes nothing.
            let asked = time_key(
                translated,
                dash.time_range(),
                &dash.dashboard().time,
                now_ms(),
            );
            if let Some(range) = asked {
                dash.set_time_range(range);
                continue;
            }
            // The program owns each key of a variable as well. thunderbox
            // consumes none of them, and `thunderbox_dashboard::variable_key`
            // holds the rule alone. That function asks for no selection when
            // the key changes nothing.
            if let Some(selection) = variable_key(translated, dash.variables()) {
                dash.set_variables(selection);
                continue;
            }
            // The program owns the keys of a row panel as well, and
            // `thunderbox_dashboard::row_key` holds the rule alone. That function
            // answers only while a person has selected a row panel.
            if let Some(row) = row_key(translated, view.selected(), dash.dashboard()) {
                dash.toggle_row(row);
                continue;
            }
            let _: Consumed = view.handle_key(translated, dash.dashboard(), &snapshot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thunderbox_core::PanelKind;

    #[test]
    fn the_dashboard_inside_the_program_is_correct() {
        let dash = load_dashboard(None).expect("The default dashboard must be correct.");
        assert_eq!(dash.title, "thunderbox");
        assert_eq!(dash.panels.len(), 7);
    }

    #[test]
    fn the_dashboard_inside_the_program_reads_this_machine() {
        let dash = load_dashboard(None).expect("The default dashboard must be correct.");
        assert_eq!(
            dash.sources.get("machine").map(|s| s.kind.as_str()),
            Some("sysinfo"),
            "The program must show real values when a person names no file."
        );
        for panel in &dash.panels {
            for target in &panel.targets {
                assert_eq!(
                    target.source, "machine",
                    "Every panel of the default dashboard must read this machine."
                );
            }
        }
    }

    /// The default dashboard is the demonstration of thunderbox. A person who
    /// runs the program with no file must see every type of panel that shows
    /// data.
    ///
    /// A row panel shows no data, and the list below leaves it out. Each row
    /// panel takes one line of a terminal, and the default dashboard must fit a
    /// small terminal. `dashboards/rows.toml` demonstrates the row panels
    /// instead.
    #[test]
    fn the_dashboard_inside_the_program_shows_every_type_of_panel() {
        let dash = load_dashboard(None).expect("The default dashboard must be correct.");
        for kind in [
            PanelKind::TimeSeries,
            PanelKind::Stat,
            PanelKind::Gauge,
            PanelKind::BarGauge,
            PanelKind::Table,
            PanelKind::Top,
        ] {
            assert!(
                dash.panels.iter().any(|p| p.kind == kind),
                "The default dashboard must hold a panel of the type {kind:?}."
            );
        }
    }

    /// A dashboard that holds a row panel adds the key that collapses it. The
    /// hint follows the list of the keys, and the row drops it when it does not
    /// fit.
    #[test]
    fn the_status_row_adds_the_key_of_a_row_panel_while_it_fits() {
        let keys = status_text(0, 0, None, None, false, None, 120);
        assert_eq!(
            with_row_key(keys.clone(), true, 120),
            format!("{KEYS}  {ROW_KEY}")
        );
        assert_eq!(
            with_row_key(keys.clone(), false, 120),
            KEYS,
            "A dashboard with no row panel keeps the row that it had."
        );
        assert_eq!(
            with_row_key(keys, true, 80),
            KEYS,
            "The list of 75 columns and the hint do not fit 80 columns."
        );
        let error = status_text(1, 0, None, None, false, None, 40);
        assert_eq!(
            with_row_key(error.clone(), true, 200),
            error,
            "A row that dropped the list of the keys takes no hint either."
        );
    }

    /// The status row must stay quiet while no panel holds an error and the
    /// numbers are new. A row that changes on each refresh would draw the eye
    /// away from the panels.
    #[test]
    fn the_status_row_shows_the_keys_alone_while_no_panel_holds_an_error() {
        let text = status_text(0, 0, None, Some(Duration::from_secs(0)), false, None, 80);
        assert_eq!(
            text, KEYS,
            "A row that holds no other sentence must show the keys."
        );
    }

    /// A terminal that is too small holds panels that nobody can see. thunderbox
    /// asks a data source for nothing for such a panel, so that panel also
    /// holds no data. The row must name the count, because a person who reads
    /// an empty screen learns nothing without it.
    #[test]
    fn the_status_row_reports_the_panels_that_the_terminal_is_too_small_for() {
        let one = status_text(0, 1, None, None, false, None, 120);
        assert!(
            one.starts_with("The terminal is too small for 1 panel."),
            "A reader must learn why a panel is missing. Text: {one}"
        );
        let three = status_text(0, 3, None, None, false, None, 120);
        assert!(
            three.starts_with("The terminal is too small for 3 panels."),
            "The row must count the panels. Text: {three}"
        );
        let none = status_text(0, 0, None, None, false, None, 120);
        assert_eq!(
            none, KEYS,
            "A row of a terminal that drops no panel must show the keys alone."
        );
    }

    #[test]
    fn the_status_row_reports_a_set_of_queries_that_takes_a_long_time() {
        let text = status_text(0, 0, None, Some(Duration::from_secs(0)), true, None, 120);
        assert!(
            text.starts_with("The dashboard waits for an answer."),
            "A reader must learn that a set of queries runs. Text: {text}"
        );
    }

    #[test]
    fn the_status_row_reports_numbers_that_are_old() {
        let text = status_text(0, 0, None, Some(Duration::from_secs(30)), false, None, 120);
        assert!(
            text.starts_with("The numbers are 30 seconds old."),
            "A reader must learn how old the numbers are. Text: {text}"
        );
    }

    #[test]
    fn the_status_row_says_nothing_about_numbers_that_are_new() {
        let text = status_text(0, 0, None, Some(Duration::from_secs(1)), false, None, 120);
        assert_eq!(text, KEYS, "A new answer needs no sentence of its own.");
    }

    #[test]
    fn the_status_row_counts_one_panel_that_holds_an_error() {
        let text = status_text(1, 0, None, None, false, None, 120);
        assert!(
            text.starts_with("1 panel holds an error."),
            "The sentence must name one panel. Text: {text}"
        );
        assert!(
            status_text(2, 0, None, None, false, None, 120).starts_with("2 panels hold an error.")
        );
    }

    /// The row is one cell high. `CLAUDE.md` states that a text which does not
    /// fit gives way, in the section `The rule for a short label inside a
    /// panel`. This row drops a whole sentence, and it shortens no sentence of
    /// its own.
    #[test]
    fn the_status_row_drops_the_sentence_that_a_person_needs_least() {
        let text = status_text(2, 0, None, Some(Duration::from_secs(30)), true, None, 40);
        assert_eq!(
            text, "2 panels hold an error.",
            "A narrow row must keep the most important sentence."
        );
        let wider = status_text(2, 0, None, Some(Duration::from_secs(30)), true, None, 62);
        assert_eq!(
            wider, "2 panels hold an error.    The dashboard waits for an answer.",
            "A wider row must hold the sentence that comes next."
        );
    }

    /// A person who moves the range meets a panel that holds no data. The data
    /// source of this machine holds nothing from before the program started.
    /// The row is the one place that can say why.
    #[test]
    fn the_status_row_names_a_window_that_ended_before_the_present_moment() {
        let window = TimeWindow {
            from: 0,
            to: 300_000,
            now: 3_900_000,
        };
        let text = status_text(0, 0, None, None, false, Some(window), 120);
        assert!(
            text.starts_with("The panels show 5 minutes that ended 1 hour ago."),
            "A reader must learn which window the panels show. Text: {text}"
        );
    }

    #[test]
    fn the_status_row_says_nothing_about_a_range_that_follows_the_clock() {
        let text = status_text(0, 0, None, None, false, None, 120);
        assert_eq!(text, KEYS, "An ordinary range needs no sentence.");
    }

    /// The list of the keys does not fit beside the sentence that names a
    /// window. A person who reads that sentence needs the key that returns the
    /// range of the dashboard, so the row keeps that one hint.
    #[test]
    fn the_status_row_keeps_the_key_of_the_range_beside_the_sentence() {
        let window = TimeWindow {
            from: 0,
            to: 300_000,
            now: 3_900_000,
        };
        let text = status_text(0, 0, None, None, false, Some(window), 80);
        assert!(
            text.starts_with("The panels show 5 minutes that ended 1 hour ago."),
            "A reader must learn which window the panels show. Text: {text}"
        );
        assert!(
            text.ends_with(RANGE_KEY),
            "A reader must learn the key that brings the range back. Text: {text}"
        );
        assert!(
            text.chars().count() <= 80,
            "The row must fit the terminal. Text: {text}"
        );
    }

    /// The whole list of the keys must fit a terminal of 80 columns. The row
    /// would otherwise hold no key at all on such a terminal. The list ends
    /// with the hint that the row keeps by itself, so a reader meets the same
    /// words in both places.
    #[test]
    fn the_list_of_the_keys_fits_a_terminal_of_eighty_columns() {
        assert!(
            KEYS.chars().count() <= 80,
            "The list holds {} characters.",
            KEYS.chars().count()
        );
        assert!(
            KEYS.ends_with(RANGE_KEY),
            "The two texts must name that key with the same words. {KEYS}"
        );
    }

    /// The program owns every key of the time range. This test presses each of
    /// them, and it reads the range that the key asks for.
    #[test]
    fn each_key_of_the_time_range_moves_the_range() {
        const NOW: thunderbox_core::Timestamp = 1_000_000;
        let declared = thunderbox_core::TimeRange::default();
        let press = |code, range: &thunderbox_core::TimeRange| {
            time_key(translate(code), range, &declared, NOW).expect("The key must move the range.")
        };

        let back = press(KeyCode::Left, &declared);
        assert_eq!(back.resolve(NOW), (NOW - 450_000, NOW - 150_000));
        assert!(
            !back.moves_with_the_clock(),
            "A window that a person moved must stand still."
        );

        let wider = press(KeyCode::Char('-'), &declared);
        assert_eq!(wider.span_ms(NOW), 600_000);
        let narrower = press(KeyCode::Char('+'), &declared);
        assert_eq!(narrower.span_ms(NOW), 150_000);

        assert_eq!(press(KeyCode::Char('0'), &back), declared);
        assert_eq!(press(KeyCode::Char('='), &back), declared);
        assert!(
            press(KeyCode::Right, &back).moves_with_the_clock(),
            "One key must bring a person back to the present moment."
        );
    }

    /// thunderbox keeps its own three keys, and it uses none of the keys that
    /// move the time range.
    #[test]
    fn a_key_of_the_dashboard_moves_no_time_range() {
        let declared = thunderbox_core::TimeRange::default();
        for code in [KeyCode::Tab, KeyCode::BackTab, KeyCode::Char('m')] {
            assert_eq!(
                time_key(translate(code), &declared, &declared, 1_000_000),
                None
            );
        }
    }

    /// Returns the state of one variable that states two options.
    fn one_variable(name: &str) -> Variables {
        Variables::new(&[thunderbox_core::Variable {
            name: name.to_string(),
            label: None,
            options: vec!["web".into(), "db".into()],
            query: None,
            current: None,
        }])
    }

    /// The default dashboard declares no variable. A person who runs the
    /// program with no file must therefore see the picture that this project
    /// recorded.
    #[test]
    fn the_dashboard_inside_the_program_declares_no_variable() {
        let dash = load_dashboard(None).expect("The default dashboard must be correct.");
        assert!(dash.variables.is_empty());
        assert_eq!(
            variable_row_text(&Variables::new(&dash.variables), 80),
            "",
            "The program must draw no row for a dashboard that declares no variable."
        );
    }

    #[test]
    fn the_row_of_the_variables_names_each_one_and_lists_the_keys() {
        let text = variable_row_text(&one_variable("job"), 80);
        assert_eq!(text, "[job: web]    v next variable  [] value");
    }

    /// `CLAUDE.md` states the rule for a text that does not fit, in the
    /// section `The rule for a short label inside a panel`. The variables
    /// come first, because a person needs the value more than the key.
    #[test]
    fn the_row_of_the_variables_drops_the_keys_on_a_narrow_terminal() {
        let text = variable_row_text(&one_variable("job"), 20);
        assert_eq!(text, "[job: web]");
    }

    /// The default dashboard declares no variable, so the row of the variables
    /// takes no line. Every line that the panels held before this program drew
    /// such a row must therefore stay with the panels.
    #[test]
    fn a_dashboard_that_declares_no_variable_leaves_every_line_to_the_panels() {
        let area = ratatui::layout::Rect::new(0, 0, 80, 24);
        let [row, body, status] = Layout::vertical([
            Constraint::Length(0),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .areas(area);
        let [body_before, status_before] =
            Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(area);
        assert_eq!(row.height, 0, "The row of the variables must take no line.");
        assert_eq!(body, body_before, "The panels must hold the same lines.");
        assert_eq!(status, status_before);
    }

    /// A variable that has no choice needs a sentence, and the row of the
    /// variables holds short labels alone. The status row is the one place of
    /// this program that holds a sentence.
    #[test]
    fn the_status_row_names_a_variable_that_has_no_choice() {
        let text = status_text(
            0,
            0,
            Some("The variable \"job\" has no choice. The data source \"prom\" did not answer."),
            None,
            false,
            None,
            120,
        );
        assert!(
            text.starts_with("The variable \"job\" has no choice."),
            "A reader must learn why a variable holds no value. Text: {text}"
        );
    }

    /// The program owns every key of a variable. This test presses each of
    /// them, and it reads the selection that the key asks for.
    #[test]
    fn each_key_of_a_variable_changes_the_selection() {
        let variables = one_variable("job");
        let press = |code, variables: &Variables| {
            variable_key(translate(code), variables).expect("The key must change the selection.")
        };
        let ahead = press(KeyCode::Char(']'), &variables);
        assert_eq!(ahead.states()[0].value(), Some("db"));
        assert_eq!(
            press(KeyCode::Char('['), &variables).states()[0].value(),
            Some("db")
        );
        assert_eq!(
            variable_key(translate(KeyCode::Char('v')), &variables),
            None,
            "The dashboard declares one variable, so the cursor cannot move."
        );
    }

    /// thunderbox keeps its own three keys, and it uses none of the keys of a
    /// variable. The keys of the time range are also its own.
    #[test]
    fn a_key_of_a_variable_moves_no_time_range() {
        let declared = thunderbox_core::TimeRange::default();
        for code in [KeyCode::Char('v'), KeyCode::Char('['), KeyCode::Char(']')] {
            assert_eq!(
                time_key(translate(code), &declared, &declared, 1_000_000),
                None
            );
        }
    }

    #[test]
    fn the_help_text_lists_every_key_of_a_variable() {
        for row in [
            "    v         Select the next variable of the dashboard.",
            "    ]         Take the choice that follows, on the variable that you selected.",
            "    [         Take the choice before it, on the variable that you selected.",
        ] {
            assert!(
                HELP.contains(row),
                "The help text must hold this row. {row}"
            );
        }
    }

    /// The test reads the whole row of each key. A row that holds the name of
    /// the key alone would also match another row, such as the row of the
    /// option `-h, --help`.
    #[test]
    fn the_help_text_lists_every_key_of_the_time_range() {
        for row in [
            "    Left      Move the time range back by half of its width.",
            "    Right     Move the time range forward by half of its width.",
            "    -         Make the time range twice as wide.",
            "    +         Make the time range half as wide.",
            "    0, =      Return to the time range that the dashboard states.",
        ] {
            assert!(
                HELP.contains(row),
                "The help text must hold this row. {row}"
            );
        }
    }

    #[test]
    fn the_help_option_writes_a_text_and_stops() {
        let command = parse_arguments(vec!["--help".into()]);
        match command {
            Command::Write(text) => assert!(text.contains("Usage")),
            Command::Run(_) => panic!("The program must not start the terminal."),
        }
    }

    #[test]
    fn a_file_name_reaches_the_program() {
        match parse_arguments(vec!["board.toml".into()]) {
            Command::Run(Some(path)) => assert_eq!(path, PathBuf::from("board.toml")),
            _ => panic!("The program must read the name of the file."),
        }
    }

    #[test]
    fn a_file_that_is_not_there_returns_an_error_that_names_it() {
        let error = load_dashboard(Some(PathBuf::from("/no/such/board.toml")))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("board.toml"),
            "The message is wrong. {error}"
        );
    }
}
