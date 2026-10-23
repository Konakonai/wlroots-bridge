//! CLI contract v1 checks (docs/computer-use-bridge-contract.md in
//! claude-desktop-extra). Runs the real binary with a scrubbed environment and
//! no Wayland compositor, so it needs no sway and works in plain `cargo test`.

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "wlroots-bridge-contract-{}-{tag}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> Output {
    let home = scratch("home");
    Command::new(env!("CARGO_BIN_EXE_wlroots-bridge"))
        .args(args)
        .env_clear()
        .env("HOME", &home)
        .env("XDG_STATE_HOME", home.join("state"))
        .env("XDG_RUNTIME_DIR", home.join("run"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .output()
        .expect("spawn wlroots-bridge")
}

fn stdout_json(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).expect("stdout is one JSON value")
}

#[test]
fn version_format() {
    let out = run(&["--version"]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        text.trim(),
        format!("wlroots-bridge {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn doctor_without_compositor_exits_zero_with_required_keys() {
    let out = run(&["doctor"]);
    assert_eq!(out.status.code(), Some(0));
    let v = stdout_json(&out);
    assert_eq!(v["contract"], 1);
    assert_eq!(v["bridge"], "wlroots-bridge");
    assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(v["backend"], "wlroots");
    for key in [
        "session_match",
        "backend_reachable",
        "session_supported",
        "restore_token_present",
        "daemon_running",
    ] {
        assert_eq!(v[key], false, "{key} must be false without a compositor");
    }
    assert!(v["backend_error"].is_string());
    assert!(v["restore_token_path"].is_null());
    assert!(v["session_socket"].is_null());
}

#[test]
fn session_end_is_exact() {
    let out = run(&["session-end"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        r#"{"ok":true,"ended":false,"daemon":"none"}"#
    );
}

#[test]
fn session_start_is_noop() {
    let out = run(&["session-start"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        stdout_json(&out),
        serde_json::json!({"ok": true, "session": "noop"})
    );
}

#[test]
fn forget_session_is_exact_and_has_no_side_effects() {
    let out = run(&["forget-session"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        r#"{"ok":true,"ended":false,"daemon":"none","token_removed":false,"token_path":null}"#
    );
    let home = scratch("home");
    assert!(
        !home.join("state").exists(),
        "forget-session must not create the state dir"
    );
    assert!(
        !home.join("run").exists(),
        "forget-session must not create the runtime dir"
    );
}

#[test]
fn help_exits_zero() {
    for sub in ["screens", "windows", "session-start"] {
        let out = run(&[sub, "--help"]);
        assert_eq!(out.status.code(), Some(0), "{sub} --help");
    }
}

#[test]
fn negative_coordinates_parse() {
    // `--help` after the flags short-circuits before any Wayland connection;
    // a parse error would exit 2.
    let cases: &[&[&str]] = &[
        &["pointer-move", "--x", "-5", "--y", "-5", "--help"],
        &["pointer-click", "--x", "-5", "--y", "-5", "--help"],
        &[
            "pointer-scroll",
            "--x",
            "-5",
            "--y",
            "-5",
            "--dx",
            "-1",
            "--dy",
            "-2",
            "--help",
        ],
        &[
            "pointer-drag",
            "--from-x",
            "-1",
            "--from-y",
            "-2",
            "--to-x",
            "-3",
            "--to-y",
            "-4",
            "--help",
        ],
        &[
            "zoom", "--x", "-100", "--y", "-50", "--w", "10", "--h", "10", "--help",
        ],
        &["app-under-point", "--x", "-5", "--y", "-5", "--help"],
    ];
    for args in cases {
        let out = run(args);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn negative_coordinates_reach_the_backend() {
    // Without --help the parse must succeed and the run fail at the Wayland
    // connection (exit 1), never as a usage error (exit 2).
    let out = run(&["pointer-move", "--x", "-5", "--y", "-5"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
}

#[test]
fn bogus_subcommand_exits_two() {
    let out = run(&["definitely-not-a-command"]);
    assert_eq!(out.status.code(), Some(2));
}
