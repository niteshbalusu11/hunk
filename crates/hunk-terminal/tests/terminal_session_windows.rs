#![cfg(target_os = "windows")]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use hunk_terminal::{TerminalEvent, TerminalSpawnRequest, spawn_terminal_session};

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn terminal_session_runs_cmd_shell_commands() {
    let request = TerminalSpawnRequest::new(repo_root(), "echo windows-cmd-ok".to_string())
        .with_shell_program("cmd.exe");
    let (_handle, event_rx) =
        spawn_terminal_session(request).expect("terminal session should start");

    let events = collect_events_until_exit(&event_rx);
    assert!(output_text(&events).contains("windows-cmd-ok"));
}

#[test]
fn terminal_session_runs_powershell_commands() {
    let request = TerminalSpawnRequest::new(
        repo_root(),
        "Write-Output 'windows-powershell-ok'".to_string(),
    )
    .with_shell_program("powershell.exe");
    let (_handle, event_rx) =
        spawn_terminal_session(request).expect("terminal session should start");

    let events = collect_events_until_exit(&event_rx);
    assert!(output_text(&events).contains("windows-powershell-ok"));
}

#[test]
fn terminal_session_runs_pwsh_commands_when_available() {
    if !command_available("pwsh.exe") {
        return;
    }

    let request =
        TerminalSpawnRequest::new(repo_root(), "Write-Output 'windows-pwsh-ok'".to_string())
            .with_shell_program("pwsh.exe");
    let (_handle, event_rx) =
        spawn_terminal_session(request).expect("terminal session should start");

    let events = collect_events_until_exit(&event_rx);
    assert!(output_text(&events).contains("windows-pwsh-ok"));
}

fn collect_events_until_exit(
    event_rx: &std::sync::mpsc::Receiver<TerminalEvent>,
) -> Vec<TerminalEvent> {
    let deadline = Instant::now() + TEST_TIMEOUT;
    let mut events = Vec::new();
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let event = event_rx
            .recv_timeout(remaining.min(Duration::from_millis(250)))
            .expect("expected terminal event before timeout");
        let exited = matches!(event, TerminalEvent::Exit { .. });
        events.push(event);
        if exited {
            return events;
        }
    }
    panic!("timed out waiting for terminal exit event");
}

fn output_text(events: &[TerminalEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            TerminalEvent::Output(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
            _ => None,
        })
        .collect::<String>()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(ToOwned::to_owned)
        .expect("crate should live under workspace/crates")
}

fn command_available(program: &str) -> bool {
    std::process::Command::new("where")
        .arg(program)
        .status()
        .is_ok_and(|status| status.success())
}
