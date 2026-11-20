//! Tests that run the program itself.
//!
//! These tests check what a person, or another program, sees on the output.

use std::process::Command;

/// Returns the program that cargo built for this test.
fn program() -> Command {
    Command::new(env!("CARGO_BIN_EXE_thunderbox"))
}

#[test]
fn a_dashboard_that_is_not_there_writes_nothing_to_the_output() {
    let out = program()
        .arg("/no/such/dashboard.toml")
        .output()
        .expect("The test must be able to run the program.");
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

    let out = program()
        .arg(&path)
        .output()
        .expect("The test must be able to run the program.");
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
    let out = program()
        .arg("--help")
        .output()
        .expect("The test must be able to run the program.");
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
    let out = program()
        .arg("--version")
        .output()
        .expect("The test must be able to run the program.");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.starts_with("thunderbox "), "The text is wrong. {text}");
}
