//! The thunderbox program.
//!
//! This program owns the terminal and the event loop. It drives
//! `thunderbox-dashboard`, which runs the queries. The same runtime serves this
//! program, and another program that holds a thunderbox dashboard. Only the
//! owner of the terminal changes. A server for SSH does not exist yet, and ADR
//! 0013 in `docs/contributor/adr` states the plan for it.

use std::borrow::Cow;
use std::ffi::OsString;
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

mod sources;

use anyhow::{Context, Result, bail};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Size};
use ratatui::widgets::Paragraph;
use thunderbox_core::{Dashboard, PanelKind, TimeRange, TimeWindow, now_ms};
use thunderbox_dashboard::{
    Consumed, DashboardRuntime, DashboardView, Key, Snapshot, Variables, row_key, time_key,
    variable_key,
};
use thunderbox_widgets::{format_held_window, format_held_window_short, format_window};

/// The dashboard that the program shows when a person names no file.
const DEFAULT_DASHBOARD: &str = include_str!("default.toml");

/// The text that the program writes for the `--help` option.
const HELP: &str = "\
thunderbox is a dashboard platform for the terminal.

The dashboard inside the program reads this machine. It draws the processor,
each core, the memory, the network, and the processes.

Usage:
    thunderbox [OPTION] [--] [FILE]

    The argument -- ends the options, and each argument after it is a file
    name. A file name that starts with a dash needs ./ in front of it, or --
    before it.

Arguments:
    FILE    The dashboard to show, as a TOML file. The program shows the
            state of this machine when a person names no file.

Options:
    -h, --help       Write this text and stop.
    -V, --version    Write the version and stop.

Keys:
    q, Esc    Stop the program.
    Ctrl+C    Stop the program.
    Tab       Select the next panel.
    Shift+Tab Select the panel before.
    m         Change the glyph that a chart draws a point with.
    r         Run the queries again.
    Left      Move the time range back by half of its width.
    Right     Move the time range forward by half of its width.
    -         Make the time range twice as wide.
    +         Make the time range half as wide.
    0, =      Return to the time range that the dashboard states.
    f         Hold a time range that follows the clock behind, or make a held one follow it.
    v         Select the next variable of the dashboard.
    ]         Take the choice that follows, on the variable that you selected.
    [         Take the choice before it, on the variable that you selected.
    Enter     Collapse or expand the row panel that you selected.
    Space     Do what Enter does.
";

/// The part of the sentence of a window that follows the clock behind, which
/// names the two keys that change that window.
///
/// The key `f` holds the window, and the key `0` returns the range of the
/// dashboard.
const FOLLOWING_HINT: &str = "Press f to hold, or 0 to return.";

/// The short form of [`FOLLOWING_HINT`].
///
/// The row draws this form only when the complete sentence does not fit.
/// `CLAUDE.md` lists it in section 1.5.
const FOLLOWING_HINT_SHORT: &str = "f holds, 0 returns.";

/// The keys that the status row lists.
///
/// The row writes a short hint for each key, and it writes two spaces between
/// two hints. A reader then reads the list as one part, and the whole list
/// fits a terminal of 80 columns. The `--help` text and
/// `docs/user/reference/command-line.md` hold the complete sentence for each
/// key.
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
/// the list fits. The `--help` text and `docs/user/reference/command-line.md`
/// hold the complete sentence for each key.
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
///
/// [`parse_arguments`] returns this value when it can read the command line.
#[derive(Debug, PartialEq)]
enum Command {
    /// Show a dashboard. The program reads the named file, or it uses the
    /// default dashboard when the command line names no file. The path keeps
    /// each byte of the argument, so a name that is not text reaches the file
    /// system unchanged.
    Run(Option<PathBuf>),
    /// Write a text to the standard output and stop. The options `--help` and
    /// `--version` give this command.
    Write(String),
}

/// A command line that the program cannot read.
#[derive(Debug, PartialEq)]
enum ArgumentError {
    /// An argument starts with a dash, and the program knows no such option.
    UnknownOption(OsString),
    /// The command line names a second file. The program shows one dashboard.
    SecondFile(PathBuf),
}

impl fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOption(option) => write!(
                f,
                "The program does not know the option {}. Run thunderbox --help to read the \
                 list of the options.",
                option.to_string_lossy()
            ),
            Self::SecondFile(path) => write!(
                f,
                "The command line names a second file, {}. The program shows one dashboard \
                 only.",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ArgumentError {}

/// Reads the arguments of the command line.
///
/// The function reads the arguments from left to right. The first argument
/// that decides the result stops the function:
///
/// - `-h` or `--help` gives the help text.
/// - `-V` or `--version` gives the version.
/// - Any other argument that starts with a dash is an unknown option. The
///   function returns an error that names it.
/// - A second file name returns an error that names that file.
///
/// The first file name decides nothing, so `board.toml --help` still gives the
/// help text. The argument `--` ends the options. Each argument after it is a
/// file name, even when it starts with a dash. A file name that starts with a
/// dash therefore needs `./` in front of it, or `--` before it.
///
/// The function takes each argument as an `OsString`, so it never fails on a
/// name that is not text.
fn parse_arguments(arguments: Vec<OsString>) -> Result<Command, ArgumentError> {
    let mut file: Option<PathBuf> = None;
    let mut options_ended = false;
    for argument in arguments {
        if !options_ended {
            if argument == "--" {
                options_ended = true;
                continue;
            }
            if argument == "-h" || argument == "--help" {
                return Ok(Command::Write(HELP.to_string()));
            }
            if argument == "-V" || argument == "--version" {
                return Ok(Command::Write(format!(
                    "thunderbox {}\n",
                    env!("CARGO_PKG_VERSION")
                )));
            }
            if argument.as_encoded_bytes().starts_with(b"-") {
                return Err(ArgumentError::UnknownOption(argument));
            }
        }
        if file.is_some() {
            return Err(ArgumentError::SecondFile(PathBuf::from(argument)));
        }
        file = Some(PathBuf::from(argument));
    }
    Ok(Command::Run(file))
}

/// Writes a text to the output, and then flushes the output.
///
/// A closed pipe is not a fault. The program that reads the output closed the
/// pipe, as `head` does after its first lines, and that program has every
/// byte that it wants. The function therefore returns `Ok` for a closed pipe.
///
/// # Errors
///
/// Returns an error for each other fault of the write or of the flush. A full
/// disk is an example.
fn write_text(output: &mut impl Write, text: &str) -> Result<()> {
    match output
        .write_all(text.as_bytes())
        .and_then(|()| output.flush())
    {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        written => written.context("The program cannot write the text to the standard output."),
    }
}

fn main() -> Result<()> {
    match parse_arguments(std::env::args_os().skip(1).collect())? {
        Command::Write(text) => write_text(&mut io::stdout().lock(), &text),
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
/// that holds a fault therefore writes a message and stops.
///
/// The program then checks that its standard output is a terminal. When the
/// output is a file or a pipe, this function returns an error, and the program
/// returns 1. The program therefore sends no control sequence to a file, or to
/// a program that reads the output of this one. ADR 0071 in
/// `docs/contributor/adr` states why the check reads the output, and not the
/// input.
async fn run(path: Option<PathBuf>) -> Result<()> {
    let dashboard = load_dashboard(path)?;
    // The dashboard names each data source that it needs, and the `sources`
    // module makes them. A person can therefore mix real values and test
    // values on one page.
    let sources = sources::build(&dashboard)
        .context("The program cannot make the data sources of the dashboard.")?;

    let mut dash = DashboardRuntime::new(dashboard, sources, tokio::runtime::Handle::current())
        .context("The program cannot start the dashboard.")?;

    use std::io::IsTerminal;

    // A redirect of the output gives the program a file or a pipe in place of a
    // terminal. A dashboard there reaches no person, so the program returns an
    // error.
    if !std::io::stdout().is_terminal() {
        bail!(
            "The program draws a dashboard only on a terminal. The standard output of \
             the program is not a terminal."
        );
    }

    // The program owns the terminal from this line to the call of restore. Each
    // call to try_init has one call to restore that answers it.
    let mut terminal = match ratatui::try_init() {
        Ok(terminal) => terminal,
        Err(error) => {
            // A terminal can accept raw mode, and it can then refuse the
            // alternate screen. The restore returns the terminal to its first
            // state in that case.
            ratatui::restore();
            return Err(error).context("The program cannot start the terminal.");
        }
    };
    let result = draw_loop(&mut terminal, &mut dash).await;
    ratatui::restore();
    result
}

/// Returns the age of the numbers that a snapshot holds.
///
/// The function returns `None` while no set of queries has answered.
fn age_of(snapshot: &Snapshot) -> Option<Duration> {
    let asked_at = snapshot.updated_at?;
    // The moment can be any number. The difference then stops at the limit of
    // the number, and it does not overflow.
    u64::try_from(now_ms().saturating_sub(asked_at))
        .ok()
        .map(Duration::from_millis)
}

/// Returns the number of cells that the sentences of the status row need.
fn row_width(parts: &[String]) -> usize {
    let gaps = GAP.chars().count() * parts.len().saturating_sub(1);
    parts.iter().map(|part| part.chars().count()).sum::<usize>() + gaps
}

/// A window that ends before the present moment, and the kind of range that
/// names it.
#[derive(Clone, Copy)]
enum Behind {
    /// The range holds a fixed moment at each end, so the window stands still.
    Held(TimeWindow),
    /// The range follows the clock, and it ends before the present moment.
    Following(TimeWindow),
}

/// Returns the window that the status row must name, with the kind of range.
///
/// The function returns nothing for a window that ends at the present moment.
fn behind(range: &TimeRange, window: Option<TimeWindow>) -> Option<Behind> {
    let window = window.filter(|window| window.to < window.now)?;
    Some(if range.moves_with_the_clock() {
        Behind::Following(window)
    } else {
        Behind::Held(window)
    })
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
/// A row that names a window in the past drops the list of the keys. It names
/// the keys that change the window inside its own sentence instead.
///
/// The function reports a refresh only when the caller states that the set of
/// queries waits. A data source that answers at once therefore writes nothing,
/// and the row does not change on each refresh.
///
/// The `behind` parameter holds the window while that window ends before the
/// present moment. A held range makes the row state the two moments of the
/// window, and the sentence stays the same while the window stands still. A
/// range that follows the clock behind makes the row state the width of the
/// window and the distance back to it. That distance stays the same too,
/// because the window moves with the clock. Each sentence names the keys that
/// change the window. The row writes the short form of a sentence only when
/// the complete sentence does not fit. It tries that form before it drops the
/// sentence. A person who reads `no data` in every panel learns from that
/// sentence why the panels are empty. A window that ends at the present moment
/// writes no sentence, because nothing is out of the ordinary.
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
    behind: Option<Behind>,
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
    // The complete sentence of a window, and its short form. The row swaps the
    // sentence for the short form when the sentence does not fit.
    let window_forms = match behind {
        Some(Behind::Held(window)) => {
            Some((format_held_window(window), format_held_window_short(window)))
        }
        Some(Behind::Following(window)) => Some((
            format!("{} {FOLLOWING_HINT}", format_window(window)),
            format!("{} {FOLLOWING_HINT_SHORT}", format_window(window)),
        )),
        None => None,
    };
    if let Some((sentence, _)) = &window_forms {
        parts.push(sentence.clone());
    }
    if let Some(seconds) = old_seconds(age) {
        parts.push(format!("The numbers are {seconds} seconds old."));
    }
    parts.push(KEYS.to_string());
    // The row drops the last part while it is too wide. A part that names a
    // window becomes the short form first. A person who reads that the window
    // stands still, or that it moves on, needs the keys that change that. The
    // short form names those keys too. The row drops the short form only when
    // it does not fit either.
    while row_width(&parts) > usize::from(width) {
        match (&window_forms, parts.last()) {
            (Some((sentence, short)), Some(last)) if last == sentence => {
                let short = short.clone();
                parts.pop();
                parts.push(short);
            }
            _ if parts.len() > 1 => {
                parts.pop();
            }
            _ => break,
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

/// The time that the loop waits for an event.
///
/// The loop wakes at least once in this time, so a key reaches the program at
/// once. A wake that finds no change draws no frame.
const WAKE: Duration = Duration::from_millis(100);

/// Returns the whole seconds that the status row states as the age of the
/// numbers.
///
/// The function returns `None` while the numbers are younger than
/// [`OLD_NUMBERS`], because the row then states no age.
///
/// The status row and the decision to draw both read this value. The age
/// grows on each wake, but this value changes once each second at most. A
/// frame that shows the raw duration would differ from the frame before it on
/// each wake.
fn old_seconds(age: Option<Duration>) -> Option<u64> {
    age.filter(|age| *age >= OLD_NUMBERS)
        .map(|age| age.as_secs())
}

/// The facts of the two rows of the program, besides the snapshot.
///
/// The row of the variables and the status row read these facts. A frame that
/// shows the same facts as the frame before it shows the same two rows.
#[derive(Debug, Clone, PartialEq)]
struct RowFacts<'a> {
    /// The variables that the row of the variables shows. A wake that draws no
    /// frame borrows them. The copy of the last frame owns them.
    variables: Cow<'a, Variables>,
    /// The sentence for the first variable that has no choice.
    ///
    /// The variables already hold this fact. The field names it on its own,
    /// because the status row shows the sentence.
    variable_error: Option<String>,
    /// `true` while a set of queries has waited for [`SLOW_ANSWER`] or longer.
    waits: bool,
    /// The age of the numbers in the form that the status row shows. See
    /// [`old_seconds`].
    old_seconds: Option<u64>,
}

impl<'a> RowFacts<'a> {
    /// Reads the facts of the two rows.
    ///
    /// The function borrows the variables, and it copies none of them.
    fn new(variables: &'a Variables, waits: bool, age: Option<Duration>) -> Self {
        Self {
            variables: Cow::Borrowed(variables),
            variable_error: variables.error(),
            waits,
            old_seconds: old_seconds(age),
        }
    }

    /// Returns the same facts with a copy of the variables.
    fn into_owned(self) -> RowFacts<'static> {
        RowFacts {
            variables: Cow::Owned(self.variables.into_owned()),
            variable_error: self.variable_error,
            waits: self.waits,
            old_seconds: self.old_seconds,
        }
    }
}

/// The facts that decide whether a wake draws a frame.
///
/// The loop reads these facts on each wake. It draws a frame only when they
/// differ from the facts of the last frame.
///
/// The other inputs of a frame need no fact of their own. Each one changes
/// only together with a fact of this value:
///
/// - The panels, the labels of the time axis, and the sentences of a window
///   read the window of the snapshot. They read no clock. The window holds the
///   moment that the refresh which made it read.
/// - The selection, the marker, and the focus of the view change only when the
///   loop applies an event.
/// - The row panels that a person collapsed belong to the snapshot.
/// - The window that the status row names comes from the time range and from
///   the snapshot. The time range changes only when the loop applies an
///   event.
/// - The count of the panels that hold an error depends on the snapshot. The
///   count of the panels that the renderer drops depends on the snapshot and
///   on the size.
///
/// Two inputs read the clock. These are the age of the numbers and the time
/// that a set of queries has waited. The row facts hold both of them, in the
/// form that the status row shows.
///
/// Add a fact here for a new input of a frame that reads the clock. Add a
/// fact also for an input that changes with no event and no new snapshot. ADR
/// 0094 in `docs/contributor/adr` states the decision.
#[derive(Debug, Clone, PartialEq)]
struct Facts<'a> {
    /// The number of new snapshots that the loop has read. The receiver of the
    /// runtime reports each change, also a change that keeps the same `Arc`.
    snapshots: u64,
    /// The size of the terminal on this wake.
    size: Size,
    /// The number of events that the loop has applied.
    events: u64,
    /// The facts of the two rows.
    rows: RowFacts<'a>,
}

impl Facts<'_> {
    /// Returns the same facts with a copy of the variables.
    fn into_owned(self) -> Facts<'static> {
        Facts {
            snapshots: self.snapshots,
            size: self.size,
            events: self.events,
            rows: self.rows.into_owned(),
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

/// What the loop learns from the events of one wake.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// An event asked the program to stop.
    Stop,
    /// The loop applied this number of events.
    Applied(u64),
}

/// Reads the events that wait, and returns them.
///
/// The function asks `next` for an event, and it passes the time that `next`
/// may wait. The first call waits for [`WAKE`]. Each later call waits for no
/// time, so the function returns when the queue holds no more events. A burst
/// of keys therefore reaches the loop in one wake.
///
/// # Errors
///
/// Returns an error when `next` returns an error.
fn read_burst(
    mut next: impl FnMut(Duration) -> io::Result<Option<Event>>,
) -> io::Result<Vec<Event>> {
    let mut events = Vec::new();
    let mut wait = WAKE;
    while let Some(event) = next(wait)? {
        events.push(event);
        wait = Duration::ZERO;
    }
    Ok(events)
}

/// Reads the events of one wake from the terminal, and returns them.
///
/// The function asks `poll` whether an event waits, and it passes the time that
/// `poll` may wait. It calls `read` only when `poll` answers `true`. The
/// function [`read_burst`] decides how long each call waits. The loop passes
/// the functions of the terminal, and a test passes a queue that it builds.
///
/// # Errors
///
/// Returns an error when `poll` or `read` returns an error.
fn read_wake(
    mut poll: impl FnMut(Duration) -> io::Result<bool>,
    mut read: impl FnMut() -> io::Result<Event>,
) -> io::Result<Vec<Event>> {
    read_burst(|wait| {
        if poll(wait)? {
            read().map(Some)
        } else {
            Ok(None)
        }
    })
}

/// Applies each event in order, and it returns at the first event that stops
/// the program.
///
/// A key that is not a press changes nothing. Any other event counts as
/// applied, because the loop cannot tell that the event changes nothing.
fn apply_events(
    events: impl IntoIterator<Item = Event>,
    dash: &mut DashboardRuntime,
    view: &mut DashboardView,
) -> Outcome {
    let mut applied = 0;
    for event in events {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Press => {}
            Event::Key(key) => {
                if apply_key(key, dash, view) {
                    return Outcome::Stop;
                }
                applied += 1;
            }
            _ => applied += 1,
        }
    }
    Outcome::Applied(applied)
}

/// Applies one key press, and returns `true` when the key stops the program.
///
/// The function reads the snapshot again for each key. An earlier key of the
/// same burst can change the snapshot, and the key `Enter` that collapses a
/// row panel is such a key. The next `Tab` must skip the panels that the row
/// panel hides now.
fn apply_key(key: KeyEvent, dash: &mut DashboardRuntime, view: &mut DashboardView) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return true;
        }
        KeyCode::Char('r') => {
            dash.refresh();
            return false;
        }
        _ => {}
    }
    let translated = translate(key.code);
    // The program owns each key of the time range. thunderbox consumes none of
    // them, and `thunderbox_dashboard::time_key` holds the rule alone. ADR 0035
    // in `docs/contributor/adr` states that rule. That function asks for no
    // range when the key changes nothing.
    let asked = time_key(
        translated,
        dash.time_range(),
        &dash.dashboard().time,
        now_ms(),
    );
    if let Some(range) = asked {
        dash.set_time_range(range);
        return false;
    }
    // The program owns each key of a variable as well. thunderbox consumes none
    // of them, and `thunderbox_dashboard::variable_key` holds the rule alone.
    // That function asks for no selection when the key changes nothing.
    if let Some(selection) = variable_key(translated, dash.variables()) {
        dash.set_variables(selection);
        return false;
    }
    // The program owns the keys of a row panel as well, and
    // `thunderbox_dashboard::row_key` holds the rule alone. That function
    // answers only while a person has selected a row panel.
    if let Some(row) = row_key(translated, view.selected(), dash.dashboard()) {
        dash.toggle_row(row);
        return false;
    }
    let snapshot = dash.snapshot();
    let _: Consumed = view.handle_key(translated, dash.dashboard(), &snapshot);
    false
}

/// Draws one frame, and returns the size of the area that holds the panels.
///
/// The frame shows the facts that the caller compared. It reads no fact of its
/// own for the two rows.
///
/// # Errors
///
/// Returns an error when the terminal cannot take the frame.
fn draw_frame(
    terminal: &mut ratatui::DefaultTerminal,
    view: &DashboardView,
    dash: &DashboardRuntime,
    snapshot: &Snapshot,
    rows: &RowFacts<'_>,
    age: Option<Duration>,
) -> io::Result<Size> {
    let dashboard = dash.dashboard();
    let mut panel_area = Size::new(0, 0);
    // A dashboard that declares no variable draws no row for one. The default
    // dashboard of this program is such a dashboard.
    let variable_height = u16::from(!rows.variables.is_empty());
    // The row reads the window that the panels draw, and not a window of its
    // own. The row and the axis of time then name one window. The row names
    // that window while it ends before the present moment, and it stays quiet
    // at every other time.
    let behind = behind(dash.time_range(), snapshot.window());
    let has_rows = dashboard
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
        view.render(dashboard, snapshot, body, frame.buffer_mut());

        if variable_height > 0 {
            let row = variable_row_text(&rows.variables, top.width);
            frame.render_widget(Paragraph::new(row), top);
        }

        let text = status_text(
            snapshot.error_count(),
            view.dropped_panels(dashboard, snapshot, body),
            rows.variable_error.as_deref(),
            age,
            rows.waits,
            behind,
            status.width,
        );
        let text = with_row_key(text, has_rows, status.width);
        frame.render_widget(Paragraph::new(text), status);
    })?;
    Ok(panel_area)
}

/// Draws the dashboard until a person stops the program.
///
/// The loop wakes at least ten times each second, so a key reaches the program
/// at once. A wake draws a frame only when a fact differs from the fact of the
/// last frame. [`Facts`] lists those facts, and [`must_draw`] decides.
///
/// Each wake follows these steps:
///
/// 1. The loop reads the size of the terminal and the snapshot.
/// 2. It draws a frame when [`must_draw`] says so.
/// 3. It reports the area, the marker, and the time to the runtime.
/// 4. It waits for an event, and then it applies each event that waits.
///
/// The loop applies every event before the next wake reads the snapshot. A
/// frame therefore never shows the state from before a key of the same burst.
async fn draw_loop(
    terminal: &mut ratatui::DefaultTerminal,
    dash: &mut DashboardRuntime,
) -> Result<()> {
    let mut view = DashboardView::new();
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
            let age = age_of(&snapshot);
            // The runtime times each set of queries from the moment when it
            // started that set. The row therefore reports a data source that
            // answers slowly, and it never reports the age of the frames of
            // this program. The snapshot of this wake must also show the wait.
            let waits = snapshot.is_refreshing()
                && dash.refresh_age().is_some_and(|age| age >= SLOW_ANSWER);
            let facts = Facts {
                snapshots: snapshot_count,
                size,
                events: event_count,
                rows: RowFacts::new(dash.variables(), waits, age),
            };
            if must_draw(last.as_ref(), &facts) {
                panel_area = draw_frame(terminal, &view, dash, &snapshot, &facts.rows, age)?;
                last = Some(facts.into_owned());
            }
        }

        // The program reports the whole area that holds the panels. thunderbox
        // divides that area onto the panels, and each panel then asks its data
        // source for the points of its own width. thunderbox asks for nothing at
        // all for a panel that it draws no picture of. A wake that draws no
        // frame reports the area of the last frame.
        dash.set_panel_area(panel_area);
        dash.set_marker(view.marker());
        // The program drives the refresh. thunderbox starts no timer of its own.
        dash.tick();

        let events = read_wake(event::poll, event::read)?;
        match apply_events(events, dash, &mut view) {
            Outcome::Stop => return Ok(()),
            Outcome::Applied(count) => event_count += count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thunderbox_core::PanelKind;

    #[test]
    fn age_of_does_not_panic_for_a_moment_at_the_limits_of_the_number() {
        let mut snapshot = Snapshot::default();
        assert_eq!(age_of(&snapshot), None, "No set of queries has answered.");
        snapshot.updated_at = Some(i64::MIN);
        assert!(age_of(&snapshot).is_some(), "The age is very large.");
        snapshot.updated_at = Some(i64::MAX);
        assert_eq!(age_of(&snapshot), None, "A moment ahead gives no age.");
    }

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
        let text = status_text(
            0,
            0,
            None,
            None,
            false,
            Some(Behind::Following(window)),
            120,
        );
        assert!(
            text.starts_with("The panels show 5 minutes that ended 1 hour ago."),
            "A reader must learn which window the panels show. Text: {text}"
        );
    }

    fn held_window() -> TimeWindow {
        TimeWindow {
            from: 50_400_000,
            to: 50_520_000,
            now: 60_000_000,
        }
    }

    #[test]
    fn the_status_row_writes_the_held_sentence_for_a_held_window() {
        let text = status_text(
            0,
            0,
            None,
            None,
            false,
            Some(Behind::Held(held_window())),
            120,
        );
        assert!(
            text.starts_with(
                "The panels hold 14:00:00 to 14:02:00 UTC. Press f to follow the clock behind, or 0 to return."
            ),
            "A reader must learn that the window stands still. Text: {text}"
        );
        assert!(
            !text.contains("ended"),
            "The sentence must hold no distance from the present moment. Text: {text}"
        );
    }

    #[test]
    fn the_status_row_writes_the_distance_for_a_window_that_follows_the_clock() {
        let text = status_text(
            0,
            0,
            None,
            None,
            false,
            Some(Behind::Following(held_window())),
            120,
        );
        assert!(
            text.starts_with("The panels show 2 minutes that ended 3 hours ago."),
            "A window that follows the clock keeps the distance. Text: {text}"
        );
    }

    #[test]
    fn the_status_row_writes_the_short_form_when_the_held_sentence_does_not_fit() {
        let text = status_text(
            0,
            0,
            None,
            None,
            false,
            Some(Behind::Held(held_window())),
            40,
        );
        assert_eq!(text, "Held 14:00:00 to 14:02:00. f follows, 0 returns.");
        let wide = status_text(
            0,
            0,
            None,
            None,
            false,
            Some(Behind::Held(held_window())),
            93,
        );
        assert!(
            wide.starts_with("The panels hold"),
            "The complete sentence must stay while it fits. Text: {wide}"
        );
        let narrow = status_text(
            0,
            0,
            None,
            None,
            false,
            Some(Behind::Held(held_window())),
            92,
        );
        assert!(
            narrow.starts_with("Held "),
            "The short form must replace a sentence that does not fit. Text: {narrow}"
        );
    }

    #[test]
    fn the_status_row_names_the_keys_of_a_window_that_follows_the_clock() {
        let text = status_text(
            0,
            0,
            None,
            None,
            false,
            Some(Behind::Following(held_window())),
            120,
        );
        assert_eq!(
            text,
            "The panels show 2 minutes that ended 3 hours ago. Press f to hold, or 0 to return."
        );
    }

    #[test]
    fn the_status_row_writes_the_short_keys_when_the_following_sentence_does_not_fit() {
        let text = status_text(
            0,
            0,
            None,
            None,
            false,
            Some(Behind::Following(held_window())),
            81,
        );
        assert_eq!(
            text,
            "The panels show 2 minutes that ended 3 hours ago. f holds, 0 returns."
        );
        let fits = status_text(
            0,
            0,
            None,
            None,
            false,
            Some(Behind::Following(held_window())),
            82,
        );
        assert!(
            fits.ends_with("Press f to hold, or 0 to return."),
            "The complete sentence must stay while it fits. Text: {fits}"
        );
    }

    #[test]
    fn the_following_sentence_does_not_change_when_only_the_present_moment_changes() {
        let at = |now| {
            // A range that follows the clock moves its window with `now`.
            let window = TimeWindow {
                from: now - 60_000_000 - 120_000,
                to: now - 60_000_000,
                now,
            };
            status_text(
                0,
                0,
                None,
                None,
                false,
                Some(Behind::Following(window)),
                120,
            )
        };
        assert_eq!(at(100_000_000), at(130_000_000));
    }

    #[test]
    fn the_status_row_writes_the_short_form_beside_the_sentence_of_an_error() {
        let text = status_text(
            1,
            0,
            None,
            None,
            false,
            Some(Behind::Held(held_window())),
            80,
        );
        assert_eq!(
            text,
            format!("1 panel holds an error.{GAP}Held 14:00:00 to 14:02:00. f follows, 0 returns."),
            "The short form fits, so the row must name the window. Text: {text}"
        );
    }

    #[test]
    fn the_status_row_writes_the_short_form_beside_the_sentence_of_a_wait() {
        let text = status_text(
            0,
            0,
            None,
            None,
            true,
            Some(Behind::Held(held_window())),
            90,
        );
        assert_eq!(
            text,
            format!(
                "The dashboard waits for an answer.{GAP}Held 14:00:00 to 14:02:00. f follows, 0 returns."
            ),
            "The short form fits, so the row must name the window. Text: {text}"
        );
    }

    #[test]
    fn the_status_row_writes_the_short_form_beside_the_sentence_of_an_error_for_a_following_window()
    {
        let text = status_text(
            1,
            0,
            None,
            None,
            false,
            Some(Behind::Following(held_window())),
            100,
        );
        assert_eq!(
            text,
            format!(
                "1 panel holds an error.{GAP}The panels show 2 minutes that ended 3 hours ago. f holds, 0 returns."
            ),
            "The short form fits, so the row must name the window. Text: {text}"
        );
    }

    #[test]
    fn the_status_row_keeps_the_complete_sentence_beside_the_sentence_of_an_error() {
        let text = status_text(
            1,
            0,
            None,
            None,
            false,
            Some(Behind::Held(held_window())),
            120,
        );
        assert!(
            text.ends_with("UTC. Press f to follow the clock behind, or 0 to return."),
            "The complete sentence fits, so the row must keep it. Text: {text}"
        );
    }

    #[test]
    fn the_held_sentence_does_not_change_when_only_the_present_moment_changes() {
        let at = |now| {
            let window = TimeWindow {
                now,
                ..held_window()
            };
            status_text(0, 0, None, None, false, Some(Behind::Held(window)), 120)
        };
        assert_eq!(at(60_000_000), at(90_000_000));
    }

    #[test]
    fn the_status_row_picks_the_sentence_by_the_kind_of_the_range() {
        use thunderbox_core::RelativeTime;
        let window = held_window();
        let held = TimeRange {
            from: RelativeTime::Absolute(window.from),
            to: RelativeTime::Absolute(window.to),
        };
        let following = TimeRange {
            from: RelativeTime::Offset(-600_000),
            to: RelativeTime::Offset(-300_000),
        };
        let live = TimeRange {
            from: RelativeTime::Offset(-300_000),
            to: RelativeTime::Now,
        };
        assert!(matches!(behind(&held, Some(window)), Some(Behind::Held(_))));
        assert!(matches!(
            behind(&following, Some(window)),
            Some(Behind::Following(_))
        ));
        assert!(
            behind(
                &live,
                Some(TimeWindow {
                    to: window.now,
                    ..window
                })
            )
            .is_none()
        );
    }

    #[test]
    fn the_status_row_says_nothing_about_a_range_that_follows_the_clock() {
        let text = status_text(0, 0, None, None, false, None, 120);
        assert_eq!(text, KEYS, "An ordinary range needs no sentence.");
    }

    /// The list of the keys does not fit beside the sentence that names a
    /// window. A person who reads that sentence needs the keys that change the
    /// window, so the row keeps the short form of those keys.
    #[test]
    fn the_status_row_keeps_the_keys_of_the_window_beside_the_sentence() {
        let window = TimeWindow {
            from: 0,
            to: 300_000,
            now: 3_900_000,
        };
        let text = status_text(0, 0, None, None, false, Some(Behind::Following(window)), 80);
        assert!(
            text.starts_with("The panels show 5 minutes that ended 1 hour ago."),
            "A reader must learn which window the panels show. Text: {text}"
        );
        assert!(
            text.ends_with(FOLLOWING_HINT_SHORT),
            "A reader must learn the keys that change the window. Text: {text}"
        );
        assert!(
            text.chars().count() <= 80,
            "The row must fit the terminal. Text: {text}"
        );
    }

    /// The whole list of the keys must fit a terminal of 80 columns. The row
    /// would otherwise hold no key at all on such a terminal.
    #[test]
    fn the_list_of_the_keys_fits_a_terminal_of_eighty_columns() {
        assert!(
            KEYS.chars().count() <= 80,
            "The list holds {} characters.",
            KEYS.chars().count()
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

        // The key `f` reaches `time_key`. It switches a held range to a range
        // that follows the clock, and it holds that range again.
        let following = press(KeyCode::Char('f'), &back);
        assert!(following.moves_with_the_clock(), "{following:?}");
        assert_eq!(following.resolve(NOW), back.resolve(NOW));
        assert_eq!(press(KeyCode::Char('f'), &following), back);
        assert_eq!(
            time_key(translate(KeyCode::Char('f')), &declared, &declared, NOW),
            None,
            "A live range already follows the clock."
        );
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
            "    f         Hold a time range that follows the clock behind, or make a held one follow it.",
        ] {
            assert!(
                HELP.contains(row),
                "The help text must hold this row. {row}"
            );
        }
    }

    /// Returns the arguments of one command line, as the program receives them.
    fn line(arguments: &[&str]) -> Vec<OsString> {
        arguments.iter().map(OsString::from).collect()
    }

    /// Returns the result of a command line that shows a dashboard.
    fn shows(file: Option<&str>) -> Result<Command, ArgumentError> {
        Ok(Command::Run(file.map(PathBuf::from)))
    }

    /// Returns the result of a command line that asks for the help text.
    fn help() -> Result<Command, ArgumentError> {
        Ok(Command::Write(HELP.to_string()))
    }

    /// Returns the result of a command line that asks for the version.
    fn version() -> Result<Command, ArgumentError> {
        Ok(Command::Write(format!(
            "thunderbox {}\n",
            env!("CARGO_PKG_VERSION")
        )))
    }

    /// Returns the result of a command line that names an unknown option.
    fn unknown(option: &str) -> Result<Command, ArgumentError> {
        Err(ArgumentError::UnknownOption(option.into()))
    }

    /// Returns the result of a command line that names a second file.
    fn second(file: &str) -> Result<Command, ArgumentError> {
        Err(ArgumentError::SecondFile(file.into()))
    }

    #[test]
    fn parse_arguments_reads_each_command_line_by_its_rules() {
        let cases = vec![
            (vec![], shows(None)),
            (vec!["board.toml"], shows(Some("board.toml"))),
            (vec!["--help"], help()),
            (vec!["-h"], help()),
            (vec!["--version"], version()),
            (vec!["-V"], version()),
            // The rows of the table of the fault.
            (vec!["--colour"], unknown("--colour")),
            (vec!["-x"], unknown("-x")),
            (
                vec!["--", "my-dashboard.toml"],
                shows(Some("my-dashboard.toml")),
            ),
            (vec!["/no/a.toml", "/no/b.toml"], second("/no/b.toml")),
            (vec!["board.toml", "--help"], help()),
            (vec!["--colour", "--help"], unknown("--colour")),
            // The argument `--` ends the options.
            (vec!["--", "-dash.toml"], shows(Some("-dash.toml"))),
            (vec!["--", "--help"], shows(Some("--help"))),
            (vec!["--", "--"], shows(Some("--"))),
            (vec!["--"], shows(None)),
            (vec!["./-dash.toml"], shows(Some("./-dash.toml"))),
            (vec!["board.toml", "--", "other.toml"], second("other.toml")),
            // The first argument that decides the result stops the parser.
            (vec!["--help", "--colour"], help()),
            (vec!["--version", "--help"], version()),
            (vec!["a.toml", "b.toml", "--help"], second("b.toml")),
            (vec!["-"], unknown("-")),
        ];
        for (arguments, expected) in cases {
            assert_eq!(
                parse_arguments(line(&arguments)),
                expected,
                "The command line {arguments:?} gives the wrong result."
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_name_that_is_not_text_reaches_the_file_system_unchanged() {
        use std::os::unix::ffi::OsStringExt;
        let name = OsString::from_vec(b"bad\xFF.toml".to_vec());
        assert_eq!(
            parse_arguments(vec![name.clone()]),
            Ok(Command::Run(Some(PathBuf::from(name))))
        );
    }

    #[test]
    fn the_error_of_an_unknown_option_names_the_option_and_the_help_option() {
        let text = unknown("--colour").unwrap_err().to_string();
        assert!(
            text.contains("--colour") && text.contains("thunderbox --help"),
            "The message is wrong. {text}"
        );
    }

    #[test]
    fn the_error_of_a_second_file_names_that_file() {
        let text = second("/no/b.toml").unwrap_err().to_string();
        assert!(text.contains("/no/b.toml"), "The message is wrong. {text}");
    }

    #[test]
    fn the_usage_block_states_that_two_dashes_end_the_options() {
        assert!(
            HELP.contains("The argument -- ends the options"),
            "The help text must state the rule of --."
        );
    }

    /// A writer that returns an error of one kind at each write.
    struct FailsAtWrite(io::ErrorKind);

    impl Write for FailsAtWrite {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// A writer that takes each write, and returns an error only at the flush.
    ///
    /// A test that holds the writer `FailsAtWrite` alone passes when the code
    /// drops the flush. This writer reports that fault.
    struct FailsAtFlush(io::ErrorKind);

    impl Write for FailsAtFlush {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from(self.0))
        }
    }

    #[test]
    fn write_text_writes_the_whole_text() {
        let mut output = Vec::new();
        write_text(&mut output, HELP).expect("A vector takes every write.");
        assert_eq!(output, HELP.as_bytes());
    }

    #[test]
    fn write_text_returns_an_error_when_the_write_fails() {
        let error = write_text(&mut FailsAtWrite(io::ErrorKind::StorageFull), HELP)
            .expect_err("A full disk is a fault.");
        assert!(
            error.to_string().contains("cannot write"),
            "The message is wrong. {error}"
        );
    }

    #[test]
    fn write_text_returns_an_error_when_only_the_flush_fails() {
        assert!(
            write_text(&mut FailsAtFlush(io::ErrorKind::StorageFull), HELP).is_err(),
            "A flush that fails is a fault."
        );
    }

    #[test]
    fn write_text_returns_ok_for_a_closed_pipe() {
        assert!(
            write_text(&mut FailsAtWrite(io::ErrorKind::BrokenPipe), HELP).is_ok(),
            "A reader that closed the pipe is not a fault of the write."
        );
        assert!(
            write_text(&mut FailsAtFlush(io::ErrorKind::BrokenPipe), HELP).is_ok(),
            "A reader that closed the pipe is not a fault of the flush."
        );
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

    /// Returns the facts of a wake with no change, and a borrowed copy of the
    /// variables.
    fn facts(variables: &Variables) -> Facts<'_> {
        Facts {
            snapshots: 1,
            size: Size::new(110, 32),
            events: 0,
            rows: RowFacts::new(variables, false, None),
        }
    }

    /// Returns `true` when a wake that reads `now` draws a frame after a frame
    /// that read `last`.
    fn draws(last: &Facts<'_>, now: &Facts<'_>) -> bool {
        must_draw(Some(last), now)
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
        let variables = one_variable("job");
        let last = facts(&variables).into_owned();
        assert!(
            !draws(&last, &facts(&variables)),
            "A wake with the same facts must draw no frame."
        );
    }

    #[test]
    fn a_new_snapshot_draws_a_frame() {
        let variables = Variables::default();
        let last = facts(&variables);
        let mut now = last.clone();
        now.snapshots += 1;
        assert!(draws(&last, &now), "A new snapshot must draw a frame.");
    }

    #[test]
    fn a_new_size_draws_a_frame() {
        let variables = Variables::default();
        let last = facts(&variables);
        let mut now = last.clone();
        now.size = Size::new(111, 32);
        assert!(draws(&last, &now), "A new width must draw a frame.");
        now.size = Size::new(110, 31);
        assert!(draws(&last, &now), "A new height must draw a frame.");
    }

    #[test]
    fn an_applied_event_draws_a_frame() {
        let variables = Variables::default();
        let last = facts(&variables);
        let mut now = last.clone();
        now.events += 1;
        assert!(draws(&last, &now), "An applied event must draw a frame.");
    }

    #[test]
    fn a_change_of_the_variables_draws_a_frame() {
        let variables = one_variable("job");
        let ahead = variable_key(translate(KeyCode::Char(']')), &variables)
            .expect("The key must change the selection.");
        let last = facts(&variables);
        let now = facts(&ahead);
        assert!(
            draws(&last, &now),
            "A new value of a variable must draw a frame."
        );
    }

    #[test]
    fn an_error_of_a_variable_draws_a_frame() {
        let variables = one_variable("job");
        let last = facts(&variables);
        let mut now = last.clone();
        now.rows.variable_error = Some("The variable \"job\" has no choice.".to_string());
        assert!(
            draws(&last, &now),
            "An error of a variable must draw a frame."
        );
    }

    #[test]
    fn a_set_of_queries_that_starts_or_stops_to_wait_draws_a_frame() {
        let variables = Variables::default();
        let last = facts(&variables);
        let mut waits = last.clone();
        waits.rows.waits = true;
        assert!(
            draws(&last, &waits),
            "A set of queries that starts to wait must draw a frame."
        );
        assert!(
            draws(&waits, &last),
            "A set of queries that stops to wait must draw a frame."
        );
    }

    #[test]
    fn a_new_age_text_draws_a_frame() {
        let variables = Variables::default();
        let at = |age: Option<Duration>| Facts {
            rows: RowFacts::new(&variables, false, age),
            ..facts(&variables)
        };
        assert!(
            draws(&at(None), &at(Some(OLD_NUMBERS))),
            "The numbers that reach the old age must draw a frame."
        );
        assert!(
            draws(
                &at(Some(Duration::from_millis(5_900))),
                &at(Some(Duration::from_millis(6_000)))
            ),
            "A new count of seconds must draw a frame."
        );
    }

    /// The age grows on each wake. The status row shows the whole seconds
    /// once the numbers are old, and it shows nothing before that. A frame
    /// that stored the raw age would differ on each wake.
    #[test]
    fn an_age_that_grows_but_keeps_the_same_text_draws_no_frame() {
        let variables = Variables::default();
        let at = |age: Option<Duration>| Facts {
            rows: RowFacts::new(&variables, false, age),
            ..facts(&variables)
        };
        assert!(
            !draws(
                &at(Some(Duration::from_millis(5_100))),
                &at(Some(Duration::from_millis(5_900)))
            ),
            "An age with the same seconds must draw no frame."
        );
        assert!(
            !draws(
                &at(Some(Duration::from_millis(100))),
                &at(Some(Duration::from_millis(4_900)))
            ),
            "The status row shows no age before the numbers are old."
        );
    }

    #[test]
    fn the_status_row_shows_the_seconds_that_the_decision_compares() {
        let text = status_text(
            0,
            0,
            None,
            Some(Duration::from_millis(7_900)),
            false,
            None,
            120,
        );
        assert!(
            text.contains("The numbers are 7 seconds old."),
            "The row must show the whole seconds of the age. Text: {text}"
        );
        assert_eq!(old_seconds(Some(Duration::from_millis(7_900))), Some(7));
    }

    /// Returns a runtime for the dashboard `text`, and the async runtime that
    /// serves it. The test must keep the async runtime alive.
    fn runtime_of(text: &str) -> (tokio::runtime::Runtime, DashboardRuntime) {
        let async_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("The test must be able to start the async runtime.");
        let dashboard = Dashboard::from_toml(text).expect("The dashboard must be correct.");
        let sources = sources::build(&dashboard).expect("The data sources must start.");
        let dash = DashboardRuntime::new(dashboard, sources, async_runtime.handle().clone())
            .expect("The runtime must start.");
        (async_runtime, dash)
    }

    /// Returns the text of a dashboard that holds four stat panels and no
    /// refresh.
    fn four_panels() -> String {
        let mut text = "title = \"burst\"\n[source.test]\nkind = \"testdata\"\n".to_string();
        for index in 0..4 {
            text.push_str(&format!(
                "[[panel]]\ntitle = \"p{index}\"\nkind = \"stat\"\n\
                 grid = {{ x = {}, y = 0, w = 6, h = 4 }}\n\
                 [[panel.target]]\nsource = \"test\"\nexpr = \"ramp\"\n",
                index * 6
            ));
        }
        text
    }

    /// Returns the text of a dashboard that holds one row panel and three stat
    /// panels below it.
    fn one_row_of_three() -> String {
        let mut text = "title = \"fold\"\n[source.test]\nkind = \"testdata\"\n\
                        [[panel]]\ntitle = \"row\"\nkind = \"row\"\n\
                        grid = { x = 0, y = 0, w = 24, h = 1 }\n"
            .to_string();
        for index in 0..3 {
            text.push_str(&format!(
                "[[panel]]\ntitle = \"p{index}\"\nkind = \"stat\"\n\
                 grid = {{ x = {}, y = 1, w = 8, h = 4 }}\n\
                 [[panel.target]]\nsource = \"test\"\nexpr = \"ramp\"\n",
                index * 8
            ));
        }
        text
    }

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    /// Returns a function that hands out the events of `queue` one at a time,
    /// as the queue of the terminal does.
    fn queue_of(events: Vec<Event>) -> impl FnMut(Duration) -> io::Result<Option<Event>> {
        let mut queue = std::collections::VecDeque::from(events);
        move |_| Ok(queue.pop_front())
    }

    #[test]
    fn a_burst_of_three_tab_keys_moves_the_selection_three_panels_and_draws_one_frame() {
        let (_async_runtime, mut dash) = runtime_of(&four_panels());
        let mut view = DashboardView::new();
        let events =
            read_burst(queue_of(vec![press(KeyCode::Tab); 3])).expect("The queue must not fail.");
        assert_eq!(
            events.len(),
            3,
            "One wake must read the whole burst, and not one key."
        );
        let outcome = apply_events(events, &mut dash, &mut view);
        assert_eq!(outcome, Outcome::Applied(3));
        assert_eq!(
            view.selected(),
            3,
            "Three keys Tab must move the selection three panels."
        );

        // The burst is one wake, so it makes one decision. That decision draws
        // one frame.
        let variables = Variables::default();
        let last = facts(&variables).into_owned();
        let mut now = facts(&variables);
        now.events += 3;
        assert!(draws(&last, &now), "The burst must draw a frame.");
        let drawn = now.into_owned();
        let mut next_wake = facts(&variables);
        next_wake.events += 3;
        assert!(
            !draws(&drawn, &next_wake),
            "The next wake must draw no second frame for the same burst."
        );
    }

    #[test]
    fn the_loop_reads_an_empty_queue_after_one_wait_of_the_wake() {
        let mut waits = Vec::new();
        let events = read_burst(|wait| {
            waits.push(wait);
            Ok(None)
        })
        .expect("The queue must not fail.");
        assert!(events.is_empty());
        assert_eq!(waits, vec![WAKE], "The loop must wait one wake, no more.");
    }

    #[test]
    fn the_loop_waits_for_the_first_event_and_reads_the_rest_with_no_wait() {
        let mut queue = std::collections::VecDeque::from(vec![press(KeyCode::Tab); 3]);
        let mut waits = Vec::new();
        let events = read_burst(|wait| {
            waits.push(wait);
            Ok(queue.pop_front())
        })
        .expect("The queue must not fail.");
        assert_eq!(events.len(), 3);
        assert_eq!(
            waits,
            vec![WAKE, Duration::ZERO, Duration::ZERO, Duration::ZERO],
            "Only the first call waits."
        );
    }

    /// The loop reads its events through `read_wake`. One call must return the
    /// whole burst that the terminal holds, and it must wait only for the
    /// first event.
    #[test]
    fn one_wake_of_the_loop_reads_a_burst_of_three_keys() {
        let queued = std::cell::Cell::new(3_u32);
        let mut waits = Vec::new();
        let events = read_wake(
            |wait| {
                waits.push(wait);
                Ok(queued.get() > 0)
            },
            || {
                queued.set(queued.get() - 1);
                Ok(press(KeyCode::Tab))
            },
        )
        .expect("The terminal must not fail.");
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
    fn a_burst_that_holds_q_stops_the_loop_at_once() {
        let (_async_runtime, mut dash) = runtime_of(&four_panels());
        let mut view = DashboardView::new();
        let events = vec![
            press(KeyCode::Tab),
            press(KeyCode::Char('q')),
            press(KeyCode::Tab),
        ];
        assert_eq!(apply_events(events, &mut dash, &mut view), Outcome::Stop);
        assert_eq!(view.selected(), 1, "The key after q must reach no panel.");
    }

    #[test]
    fn the_keys_esc_and_control_c_stop_the_loop_inside_a_burst() {
        for stop in [
            press(KeyCode::Esc),
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        ] {
            let (_async_runtime, mut dash) = runtime_of(&four_panels());
            let mut view = DashboardView::new();
            let events = vec![press(KeyCode::Tab), stop, press(KeyCode::Tab)];
            assert_eq!(apply_events(events, &mut dash, &mut view), Outcome::Stop);
            assert_eq!(view.selected(), 1);
        }
    }

    #[test]
    fn a_key_that_is_not_a_press_applies_nothing() {
        let (_async_runtime, mut dash) = runtime_of(&four_panels());
        let mut view = DashboardView::new();
        let mut release = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        let outcome = apply_events(vec![Event::Key(release)], &mut dash, &mut view);
        assert_eq!(outcome, Outcome::Applied(0));
        assert_eq!(view.selected(), 0);
    }

    /// The key `Enter` collapses a row panel, and the key `Tab` after it must
    /// skip the three panels that the row panel hides. The loop must read the
    /// snapshot again after the first key to skip them.
    #[test]
    fn a_tab_after_an_enter_in_one_burst_skips_the_panels_that_the_enter_hid() {
        let (_async_runtime, mut dash) = runtime_of(&one_row_of_three());
        let mut view = DashboardView::new();
        let events = vec![press(KeyCode::Enter), press(KeyCode::Tab)];
        assert_eq!(
            apply_events(events, &mut dash, &mut view),
            Outcome::Applied(2)
        );
        assert!(
            dash.collapsed_rows().contains(0),
            "The key Enter must collapse the row panel."
        );
        assert_eq!(
            view.selected(),
            0,
            "The key Tab must skip the hidden panels and select the row panel again."
        );
    }

    /// `DashboardRuntime::toggle_row` edits the snapshot in place while no
    /// other owner holds its `Arc`. The address of the `Arc` stays the same,
    /// so a comparison of the addresses would miss the collapse. The receiver
    /// of `DashboardRuntime::subscribe` reports it, and the loop reads that
    /// receiver.
    #[test]
    fn a_collapse_keeps_the_address_of_the_snapshot_and_the_receiver_reports_it() {
        let (async_runtime, mut dash) = runtime_of(&one_row_of_three());
        async_runtime.block_on(async { tokio::time::sleep(Duration::from_millis(300)).await });
        let mut snapshots = dash.subscribe();
        let _ = snapshots.borrow_and_update();
        let before = std::sync::Arc::as_ptr(&dash.snapshot());
        dash.toggle_row(0);
        let after = std::sync::Arc::as_ptr(&dash.snapshot());
        assert!(dash.collapsed_rows().contains(0));
        assert_eq!(before, after, "The collapse edits the snapshot in place.");
        assert!(
            snapshots.has_changed().unwrap_or(false),
            "The receiver must report the collapse."
        );
    }
}
