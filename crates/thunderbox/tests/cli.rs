//! Tests that run the program itself.
//!
//! These tests check what a person, or another program, sees on the output.
//! Each test starts the program through `run_with_limit` or through `Screen`.
//! Each of the two ways stops the program within `LIMIT`, so a fault of the
//! program cannot hold the test for ever.

use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use rustix::fs::{Mode, OFlags};
use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
use rustix::termios::{Winsize, tcsetwinsize};

/// The longest time that a test lets the program run.
const LIMIT: Duration = Duration::from_secs(30);

/// Returns the program that cargo built for this test.
///
/// The command reads no input. The program starts in a new session, and that
/// session holds no terminal of its own.
fn program() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_thunderbox"));
    command.stdin(Stdio::null());
    // The child then holds no terminal of its own. A fault of the program
    // therefore cannot put the terminal of the person who runs the tests in
    // raw mode.
    //
    // SAFETY: The function `setsid` is safe to call after a fork. The closure
    // allocates no memory.
    unsafe {
        command.pre_exec(|| {
            rustix::process::setsid()?;
            Ok(())
        });
    }
    command
}

/// Returns the path of the dashboard that reads the data source testdata.
fn dashboard() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../dashboards/rows.toml")
}

/// Reads a stream to its end in a thread of its own, and returns the thread.
///
/// The thread returns an empty vector when the stream is not there.
fn read_to_end<R: Read + Send + 'static>(stream: Option<R>) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut stream) = stream {
            let _ = stream.read_to_end(&mut bytes);
        }
        bytes
    })
}

/// Runs the command, and returns what the program wrote.
///
/// The function reads each piped stream in a thread of its own. It stops the
/// program when the program runs longer than `LIMIT`, and it then panics.
///
/// # Panics
///
/// Panics when the program does not start, or when the program runs longer
/// than `LIMIT`.
fn run_with_limit(mut command: Command) -> Output {
    let mut child = command
        .spawn()
        .expect("The test must be able to run the program.");
    // A command holds the descriptors of its streams until the test drops it.
    // An open copy of the slave keeps the master of a pseudo-terminal from its
    // end, and the reader of the master then waits for ever.
    drop(command);

    let stdout = read_to_end(child.stdout.take());
    let stderr = read_to_end(child.stderr.take());

    let start = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .expect("The test must be able to ask for the state of the program.")
        {
            break status;
        }
        if start.elapsed() >= LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "The program ran longer than {} seconds, so the test stopped it.",
                LIMIT.as_secs()
            );
        }
        thread::sleep(Duration::from_millis(20));
    };
    Output {
        status,
        stdout: stdout.join().expect("The reader of the output must end."),
        stderr: stderr.join().expect("The reader of the errors must end."),
    }
}

#[test]
fn a_dashboard_that_is_not_there_writes_nothing_to_the_output() {
    let mut command = program();
    command
        .arg("/no/such/dashboard.toml")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = run_with_limit(command);
    assert!(!out.status.success(), "The program must report the fault.");
    assert!(
        out.stdout.is_empty(),
        "The program must send no control sequence to a program that reads its \
         output. It sent {:?}.",
        String::from_utf8_lossy(&out.stdout)
    );
    let message = String::from_utf8_lossy(&out.stderr);
    assert!(
        message.contains("dashboard.toml"),
        "The message must name the file. {message}"
    );
}

#[test]
fn a_dashboard_that_holds_a_fault_writes_nothing_to_the_output() {
    let directory = std::env::temp_dir().join("thunderbox-cli-test");
    std::fs::create_dir_all(&directory).expect("The test must be able to make a directory.");
    let path = directory.join("broken.toml");
    std::fs::write(&path, "title = \"\"\n").expect("The test must be able to write a file.");

    let mut command = program();
    command
        .arg(&path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = run_with_limit(command);
    assert!(!out.status.success(), "The program must report the fault.");
    assert!(
        out.stdout.is_empty(),
        "The program must send no control sequence. It sent {:?}.",
        String::from_utf8_lossy(&out.stdout)
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_help_option_writes_the_text_to_the_output() {
    let mut command = program();
    command
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = run_with_limit(command);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Usage"), "The text is wrong. {text}");
    assert!(
        !text.contains('\u{1b}'),
        "The text must hold no control sequence."
    );
}

#[test]
fn the_version_option_writes_the_version_to_the_output() {
    let mut command = program();
    command
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = run_with_limit(command);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.starts_with("thunderbox "), "The text is wrong. {text}");
}

/// Checks that the program refuses to draw when no stream is a terminal.
#[test]
fn the_program_returns_1_when_no_stream_is_a_terminal() {
    let mut command = program();
    command
        .arg(dashboard())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = run_with_limit(command);
    assert_eq!(
        out.status.code(),
        Some(1),
        "The program must return the code 1."
    );
    assert!(
        out.stdout.is_empty(),
        "The program must write nothing to the output. It wrote {:?}.",
        String::from_utf8_lossy(&out.stdout)
    );
    let message = String::from_utf8_lossy(&out.stderr);
    assert!(
        message.contains("only on a terminal"),
        "The message must state the reason. {message}"
    );
}

/// Checks that the program refuses to draw into a file when the input is a
/// terminal.
///
/// This test exists because the test with no terminal is not enough. A program
/// that checks the input alone, or the error output alone, passes that test.
/// Here the input and the error output are a terminal, and the output is a
/// file. Only a check of the output passes this test. A program with a wrong
/// check puts the terminal in raw mode, draws into the file, and waits for a
/// key.
#[test]
fn the_program_returns_1_when_the_output_is_a_file_and_the_input_is_a_terminal() {
    let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)
        .expect("The test must be able to open a pseudo-terminal.");
    grantpt(&master).expect("The test must be able to grant the slave.");
    unlockpt(&master).expect("The test must be able to unlock the slave.");
    let name = ptsname(&master, Vec::new()).expect("The test must be able to name the slave.");
    let slave = rustix::fs::open(
        name.as_c_str(),
        OFlags::RDWR | OFlags::NOCTTY,
        Mode::empty(),
    )
    .expect("The test must be able to open the slave.");
    let slave_for_errors = slave
        .try_clone()
        .expect("The test must be able to copy the slave.");

    let path = std::env::temp_dir().join(format!(
        "thunderbox-cli-test-output-{}.txt",
        std::process::id()
    ));
    let file = std::fs::File::create(&path).expect("The test must be able to make a file.");

    let mut command = program();
    command
        .arg(dashboard())
        .stdin(Stdio::from(slave))
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(slave_for_errors));

    // The thread reads the master until the master reaches its end. The
    // function `run_with_limit` drops every copy of the slave in this process.
    let (sender, receiver) = mpsc::channel();
    let mut master = std::fs::File::from(master);
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            match master.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => bytes.extend_from_slice(&buffer[..count]),
            }
        }
        let _ = sender.send(bytes);
    });

    let out = run_with_limit(command);
    let terminal = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("The master of the pseudo-terminal must reach its end.");
    let written = std::fs::metadata(&path).map(|data| data.len());
    let _ = std::fs::remove_file(&path);

    assert_eq!(
        out.status.code(),
        Some(1),
        "The program must return the code 1."
    );
    assert_eq!(
        written.expect("The test must be able to read the file."),
        0,
        "The program must write nothing to the file."
    );
    let message = String::from_utf8_lossy(&terminal);
    assert!(
        message.contains("only on a terminal"),
        "The message must state the reason. {message:?}"
    );
}

/// Runs the program with the arguments, pipes both output streams, and returns
/// what the program wrote.
fn run_with_arguments<I, S>(arguments: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut command = program();
    command
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run_with_limit(command)
}

/// Checks that an option that the program does not know returns the code 1.
///
/// The message must name the option, and it must name `thunderbox --help`.
#[test]
fn an_unknown_option_returns_1_and_names_the_option() {
    let out = run_with_arguments(["--colour"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "The program must return the code 1."
    );
    assert!(
        out.stdout.is_empty(),
        "The program must write nothing to the output. It wrote {:?}.",
        String::from_utf8_lossy(&out.stdout)
    );
    let message = String::from_utf8_lossy(&out.stderr);
    assert!(
        message.contains("--colour"),
        "The message must name the option. {message}"
    );
    assert!(
        message.contains("thunderbox --help"),
        "The message must name the option --help. {message}"
    );
}

/// Checks that the two dashes end the options.
///
/// The program then reads the next argument as a file name. It loads the
/// dashboard, and it stops at the check of the terminal.
#[test]
fn a_file_name_after_two_dashes_reaches_the_program() {
    let out = run_with_arguments(["--", dashboard()]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "The program must return the code 1."
    );
    let message = String::from_utf8_lossy(&out.stderr);
    assert!(
        message.contains("only on a terminal"),
        "The program must load the dashboard and then check the terminal. {message}"
    );
    assert!(
        !message.contains("cannot read --"),
        "The program must not read the two dashes as a file name. {message}"
    );
}

/// Checks that a second file name returns the code 1.
///
/// The message must name the second file.
#[test]
fn a_second_file_returns_1_and_names_that_file() {
    let out = run_with_arguments(["/no/a.toml", "/no/b.toml"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "The program must return the code 1."
    );
    let message = String::from_utf8_lossy(&out.stderr);
    assert!(
        message.contains("/no/b.toml"),
        "The message must name the second file. {message}"
    );
}

/// Checks that a file name that is not text reaches the file system.
///
/// The program must not panic. It tries to read the file, and it returns an
/// error because the file is not there.
#[cfg(unix)]
#[test]
fn a_name_that_is_not_text_reaches_the_file_system() {
    use std::os::unix::ffi::OsStringExt;

    let name = std::ffi::OsString::from_vec(b"/no/bad\xFF.toml".to_vec());
    let out = run_with_arguments([name]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "The program must return the code 1, and not the code of a panic."
    );
    let message = String::from_utf8_lossy(&out.stderr);
    assert!(
        message.contains("cannot read"),
        "The program must try to read the file. {message}"
    );
    assert!(
        !message.contains("panicked"),
        "The program must not panic. {message}"
    );
}

/// Checks that a write fault of the version text returns the code 1.
///
/// The device `/dev/full` fails each write with the error that the disk is
/// full.
#[cfg(target_os = "linux")]
#[test]
fn the_version_to_a_full_device_returns_1() {
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("The test must be able to open the full device.");
    let mut command = program();
    command
        .arg("--version")
        .stdout(Stdio::from(full))
        .stderr(Stdio::piped());
    let out = run_with_limit(command);
    assert_eq!(
        out.status.code(),
        Some(1),
        "The program must return the code 1."
    );
    let message = String::from_utf8_lossy(&out.stderr);
    assert!(
        message.contains("cannot write"),
        "The message must state the fault. {message}"
    );
    assert!(
        !message.contains("panicked"),
        "The program must not panic. {message}"
    );
}

/// Checks that a closed pipe is not a fault.
///
/// The test closes the read end of the pipe before the program starts. A
/// program that treats the closed pipe as a fault returns the code 1.
#[test]
fn the_help_into_a_closed_pipe_returns_0() {
    let (reader, writer) = std::io::pipe().expect("The test must be able to make a pipe.");
    drop(reader);
    let mut command = program();
    command
        .arg("--help")
        .stdout(Stdio::from(writer))
        .stderr(Stdio::piped());
    let out = run_with_limit(command);
    assert_eq!(
        out.status.code(),
        Some(0),
        "The program must return the code 0."
    );
    assert!(
        out.stderr.is_empty(),
        "The program must write no error. It wrote {:?}.",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Runs the program on a dashboard file that holds `text`, and returns what
/// the program wrote.
///
/// The `name` parameter names the file. Each test uses its own name, so two
/// tests that run together never share a file.
fn run_on_dashboard(name: &str, text: &str) -> std::process::Output {
    let directory = std::env::temp_dir().join("thunderbox-cli-test");
    std::fs::create_dir_all(&directory).expect("The test must be able to make a directory.");
    let path = directory.join(name);
    std::fs::write(&path, text).expect("The test must be able to write a file.");
    let out = program()
        .arg(&path)
        .output()
        .expect("The test must be able to run the program.");
    let _ = std::fs::remove_file(&path);
    out
}

/// Checks that the program refused a dashboard with a message, and that it did
/// not panic. Returns the message.
///
/// The code 101 is the code that Rust gives to a panic.
fn refused_with_a_message(out: &std::process::Output) -> String {
    let message = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        !out.status.success(),
        "The program must report the fault. {message}"
    );
    assert_ne!(
        out.status.code(),
        Some(101),
        "The program panicked. {message}"
    );
    assert!(
        out.stdout.is_empty(),
        "The program must send no control sequence. It sent {:?}.",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        !message.contains("panicked"),
        "The message is the text of a panic. {message}"
    );
    message
}

#[test]
fn a_refresh_duration_that_does_not_fit_gets_an_error_and_no_panic() {
    let out = run_on_dashboard(
        "refresh-too-large.toml",
        "title = \"x\"\nrefresh = \"20000000000w\"\n",
    );
    let message = refused_with_a_message(&out);
    assert!(
        message.contains("20000000000w"),
        "The message must name the text. {message}"
    );
}

#[test]
fn a_start_that_does_not_fit_gets_an_error_and_no_panic() {
    let out = run_on_dashboard(
        "from-too-large.toml",
        "title = \"x\"\n[time]\nfrom = \"now-20000000000w\"\nto = \"now\"\n",
    );
    let message = refused_with_a_message(&out);
    assert!(
        message.contains("20000000000w"),
        "The message must name the text. {message}"
    );
}

#[test]
fn a_range_that_is_too_wide_gets_an_error_that_names_the_limit() {
    let out = run_on_dashboard(
        "range-too-wide.toml",
        "title = \"x\"\n[time]\nfrom = \"now-3651d\"\nto = \"now\"\n",
    );
    let message = refused_with_a_message(&out);
    assert!(
        message.contains("3650 days"),
        "The message must name the limit. {message}"
    );
}

/// The bytes that the backend of ratatui writes at the end of each draw.
///
/// A frame that wrote no cell holds this sequence and a hidden cursor, and
/// nothing else. The test therefore counts a frame with this sequence, also
/// when the frame changed no cell.
const END_OF_FRAME: &[u8] = b"\x1b[39m\x1b[49m\x1b[59m\x1b[0m";

/// A dashboard that never refreshes, so no new answer changes its screen.
///
/// The status row reports the age of the numbers after `OLD_NUMBERS`, which is
/// 5 seconds, so the screen then changes once each second with no key. A test
/// that uses this dashboard must end before the program has run for 5 seconds.
const QUIET_DASHBOARD: &str = r#"
title = "quiet"

[source.test]
kind = "testdata"

[[panel]]
title = "ramp"
kind = "stat"
grid = { x = 0, y = 0, w = 12, h = 4 }
[[panel.target]]
source = "test"
expr = "ramp"

[[panel]]
title = "sine"
kind = "stat"
grid = { x = 12, y = 0, w = 12, h = 4 }
[[panel.target]]
source = "test"
expr = "sine"
"#;

/// A program that runs in a pseudo-terminal, and the bytes that it wrote.
struct Screen {
    child: std::process::Child,
    master: std::fs::File,
    written: Arc<Mutex<Vec<u8>>>,
}

impl Screen {
    /// Starts the program on the dashboard file, in a pseudo-terminal of the
    /// size that the arguments give.
    ///
    /// # Panics
    ///
    /// Panics when the test cannot open the pseudo-terminal or run the program.
    fn start(dashboard: &std::path::Path, columns: u16, rows: u16) -> Self {
        let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)
            .expect("The test must be able to open a pseudo-terminal.");
        grantpt(&master).expect("The test must be able to grant the slave.");
        unlockpt(&master).expect("The test must be able to unlock the slave.");
        let name = ptsname(&master, Vec::new()).expect("The test must be able to name the slave.");
        let slave = rustix::fs::open(
            name.as_c_str(),
            OFlags::RDWR | OFlags::NOCTTY,
            Mode::empty(),
        )
        .expect("The test must be able to open the slave.");
        // The test sets the size before the program starts. The program reads
        // the size on each wake, so a later change also reaches it.
        set_size(&slave, columns, rows);
        let for_output = slave
            .try_clone()
            .expect("The test must be able to copy the slave.");
        let for_errors = slave
            .try_clone()
            .expect("The test must be able to copy the slave.");

        let mut command = program();
        command
            .arg(dashboard)
            .stdin(Stdio::from(slave))
            .stdout(Stdio::from(for_output))
            .stderr(Stdio::from(for_errors));
        let child = command
            .spawn()
            .expect("The test must be able to run the program.");
        // The command holds the descriptors of the slave until the test drops
        // it. An open copy of the slave keeps the master from its end.
        drop(command);

        let master = std::fs::File::from(master);
        let written = Arc::new(Mutex::new(Vec::new()));
        let mut reader = master
            .try_clone()
            .expect("The test must be able to copy the master.");
        let sink = Arc::clone(&written);
        thread::spawn(move || {
            let mut buffer = [0_u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => sink
                        .lock()
                        .expect("The reader must be able to lock the output.")
                        .extend_from_slice(&buffer[..count]),
                }
            }
        });
        Self {
            child,
            master,
            written,
        }
    }

    /// Returns the number of frames that the program has drawn so far.
    fn frames(&self) -> usize {
        let written = self
            .written
            .lock()
            .expect("The test must be able to lock the output.");
        written
            .windows(END_OF_FRAME.len())
            .filter(|window| *window == END_OF_FRAME)
            .count()
    }

    /// Sends bytes to the program, as a keyboard does.
    fn type_bytes(&mut self, bytes: &[u8]) {
        self.master
            .write_all(bytes)
            .expect("The test must be able to write to the master.");
    }

    /// Waits until the program has drawn more than `before` frames, and returns
    /// `true` when that happens within `limit`.
    fn draws_more_than(&self, before: usize, limit: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < limit {
            if self.frames() > before {
                return true;
            }
            thread::sleep(Duration::from_millis(20));
        }
        self.frames() > before
    }

    /// Waits until the program has drawn its first frames and then drawn
    /// nothing for `quiet`, and returns the number of frames.
    ///
    /// # Panics
    ///
    /// Panics when the program draws nothing, or when it never stops for `quiet`
    /// without a new frame.
    fn frames_after_start(&self, quiet: Duration) -> usize {
        let start = Instant::now();
        let mut count = 0;
        let mut since = Instant::now();
        while start.elapsed() < LIMIT {
            let now = self.frames();
            if now != count {
                count = now;
                since = Instant::now();
            } else if count > 0 && since.elapsed() >= quiet {
                return count;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("The program drew {count} frames and never stood still for {quiet:?}.");
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Sets the size of a pseudo-terminal.
fn set_size(terminal: impl std::os::fd::AsFd, columns: u16, rows: u16) {
    tcsetwinsize(
        terminal,
        Winsize {
            ws_row: rows,
            ws_col: columns,
            ws_xpixel: 0,
            ws_ypixel: 0,
        },
    )
    .expect("The test must be able to set the size.");
}

/// Writes the text of a dashboard to a file that only this test uses, and
/// returns the path.
fn write_dashboard(name: &str, text: &str) -> std::path::PathBuf {
    let directory = std::env::temp_dir().join("thunderbox-cli-test");
    std::fs::create_dir_all(&directory).expect("The test must be able to make a directory.");
    let path = directory.join(name);
    std::fs::write(&path, text).expect("The test must be able to write a file.");
    path
}

/// A dashboard that refreshes each second changes its numbers once each
/// second. Before the program drew a frame on each wake, it drew about ten
/// frames each second. The program must now draw a frame only when the screen
/// can change.
///
/// Each refresh can change the screen twice. The snapshot shows that a set of
/// queries runs, and the next snapshot holds the answer. The limit below is
/// therefore two frames each second, and a margin of two.
#[test]
fn an_idle_dashboard_that_refreshes_each_second_draws_about_two_frames_each_second() {
    let mut screen = Screen::start(std::path::Path::new(dashboard_variables()), 110, 32);
    thread::sleep(Duration::from_secs(1));
    let before = screen.frames();
    let seconds = 5;
    thread::sleep(Duration::from_secs(seconds));
    let frames = screen.frames() - before;
    screen.type_bytes(b"q");
    let limit = usize::try_from(2 * seconds + 2).expect("The limit must fit the number type.");
    assert!(
        frames >= 3,
        "The dashboard refreshes each second, so the screen must change. The \
         program drew {frames} frames in {seconds} seconds."
    );
    assert!(
        frames <= limit,
        "The program drew {frames} frames in {seconds} seconds. The limit is {limit}."
    );
}

/// Returns the path of the dashboard that refreshes each second and declares
/// a variable.
fn dashboard_variables() -> &'static str {
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../dashboards/variables.toml"
    )
}

#[test]
fn a_dashboard_that_never_refreshes_draws_no_frame_while_nobody_acts() {
    let path = write_dashboard("quiet-idle.toml", QUIET_DASHBOARD);
    let screen = Screen::start(&path, 80, 24);
    let started = screen.frames_after_start(Duration::from_millis(500));
    thread::sleep(Duration::from_secs(1));
    let _ = std::fs::remove_file(&path);
    assert_eq!(
        screen.frames(),
        started,
        "An idle dashboard that never refreshes must draw no frame."
    );
}

#[test]
fn a_key_reaches_the_screen_at_once() {
    let path = write_dashboard("quiet-key.toml", QUIET_DASHBOARD);
    let mut screen = Screen::start(&path, 80, 24);
    let started = screen.frames_after_start(Duration::from_millis(500));
    // The key Tab moves the selection. It starts no set of queries, so it
    // changes no snapshot, and only the event can draw the frame.
    screen.type_bytes(b"\t");
    let _ = std::fs::remove_file(&path);
    assert!(
        screen.draws_more_than(started, Duration::from_millis(1500)),
        "The key Tab moves the selection, so the program must draw a frame."
    );
}

#[test]
fn a_new_size_of_the_terminal_reaches_the_screen_at_once() {
    let path = write_dashboard("quiet-size.toml", QUIET_DASHBOARD);
    let screen = Screen::start(&path, 80, 24);
    let started = screen.frames_after_start(Duration::from_millis(500));
    set_size(&screen.master, 100, 30);
    let _ = std::fs::remove_file(&path);
    assert!(
        screen.draws_more_than(started, Duration::from_millis(1500)),
        "A new size of the terminal must draw a frame."
    );
}

#[test]
fn a_burst_of_three_keys_costs_one_frame() {
    let path = write_dashboard("quiet-burst.toml", QUIET_DASHBOARD);
    let mut screen = Screen::start(&path, 80, 24);
    let started = screen.frames_after_start(Duration::from_millis(500));
    // One write puts the three keys into the queue of the terminal together.
    // The loop must read all three in one wake, and it must draw one frame for
    // them. A loop that reads one key on each wake draws three frames.
    screen.type_bytes(b"\t\t\t");
    let _ = std::fs::remove_file(&path);
    assert!(
        screen.draws_more_than(started, Duration::from_millis(1500)),
        "The keys Tab move the selection, so the program must draw a frame."
    );
    thread::sleep(Duration::from_millis(500));
    assert_eq!(
        screen.frames() - started,
        1,
        "A burst of three keys must cost one frame."
    );
}
