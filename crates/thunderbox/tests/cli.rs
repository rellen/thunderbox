//! Tests that run the program itself.
//!
//! These tests check what a person, or another program, sees on the output.
//! Every test starts the program through `run_with_limit`, so a fault of the
//! program cannot hold the test for ever.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use rustix::fs::{Mode, OFlags};
use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};

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
