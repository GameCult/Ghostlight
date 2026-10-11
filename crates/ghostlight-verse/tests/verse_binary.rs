//! The `ghostlight-verse` command against a loopback scripted responder. No
//! real model is called and no key file is read: the responder answers the
//! OpenAI-compatible chat-completions request the local lane sends.

use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const BINARY: &str = env!("CARGO_BIN_EXE_ghostlight-verse");
const CATALOG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../ghostlight/tests/fixtures/aetheria-verse-catalog.cc"
);
const COMMIT: &str = "92b82e9aa5bb422c568c4bfc85a8f641426a41fc";
/// What the responder reports each call cost: 120 prompt and 30 completion.
const PER_CALL: u64 = 150;

fn verse(arguments: &[&str]) -> Output {
    Command::new(BINARY).args(arguments).output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn genesis(state: &Path) -> Output {
    verse(&[
        "genesis",
        "--catalog",
        CATALOG,
        "--aetheria-commit",
        COMMIT,
        "--state",
        state.to_str().unwrap(),
        "--owner-hash",
        "verse-owner",
    ])
}

/// Answers every chat-completions request with one tool call: the faction the
/// prompt names speaks its own line. Counts the requests it served.
struct Responder {
    address: SocketAddr,
    served: Arc<AtomicUsize>,
}

impl Responder {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let served = Arc::new(AtomicUsize::new(0));
        let counter = served.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                let counter = counter.clone();
                std::thread::spawn(move || serve(stream, &counter));
            }
        });
        Self { address, served }
    }

    fn url(&self) -> String {
        format!("http://{}", self.address)
    }
}

fn serve(mut stream: TcpStream, served: &AtomicUsize) {
    let mut raw = Vec::new();
    let mut chunk = [0u8; 4096];
    let (head_end, length) = loop {
        let read = stream.read(&mut chunk).unwrap_or(0);
        if read == 0 {
            return;
        }
        raw.extend_from_slice(&chunk[..read]);
        if let Some(end) = raw.windows(4).position(|window| window == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&raw[..end]).to_ascii_lowercase();
            let length = head
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            break (end + 4, length);
        }
    };
    while raw.len() < head_end + length {
        let read = stream.read(&mut chunk).unwrap_or(0);
        if read == 0 {
            return;
        }
        raw.extend_from_slice(&chunk[..read]);
    }
    let request: Value = serde_json::from_slice(&raw[head_end..head_end + length]).unwrap();
    let text = request.to_string();
    let faction = ["Adrasteia", "Brannoch"]
        .into_iter()
        .find(|name| text.contains(&format!("Decision owner:\\n{name}\\n")))
        .expect("a request names its faction");
    let sequence = served.fetch_add(1, Ordering::SeqCst);
    let reply = json!({
        "id": format!("reply-{sequence}"),
        "choices": [{
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": format!("call-{sequence}"),
                    "type": "function",
                    "function": {
                        "name": "speak",
                        "arguments": json!({ "text": format!("{faction} holds the line.") }).to_string(),
                    },
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": { "prompt_tokens": 120, "completion_tokens": 30 },
    })
    .to_string();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
        reply.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn run(state: &Path, responder: &Responder, ticks: &str, cap: &str) -> Output {
    verse(&[
        "run",
        "--state",
        state.to_str().unwrap(),
        "--ticks",
        ticks,
        "--token-cap",
        cap,
        "--tick-span-minutes",
        "60",
        "--model",
        "local/scripted",
        "--base-url",
        &responder.url(),
    ])
}

#[test]
fn binary_run_prints_acts_and_spend() {
    let state = tempfile::tempdir().unwrap();
    let made = genesis(state.path());
    assert!(made.status.success(), "{}", stderr(&made));
    let made = stdout(&made);
    assert!(made.contains("factions 2\n") && made.contains("phase Active\n"), "{made}");
    for file in ["world.cc"] {
        assert!(state.path().join(file).is_file());
    }

    let responder = Responder::start();
    let output = run(state.path(), &responder, "2", "1000000");
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);

    // Two ticks, each with both factions named by verb, and the spend as it
    // accrues against the cap.
    assert!(text.starts_with("run 1: 2 ticks of 60 minutes, cap 1000000 tokens"), "{text}");
    for tick in 0..2u64 {
        let spent = PER_CALL * 2 * (tick + 1);
        assert!(
            text.contains(&format!("tick {tick} closed: 2 cells, 2 acts, 0 refused\n")),
            "{text}"
        );
        assert!(
            text.contains(&format!(
                "  tokens: {} this tick, {spent} spent, {} remaining\n",
                PER_CALL * 2,
                1_000_000 - spent
            )),
            "{text}"
        );
    }
    for faction in ["Adrasteia", "Brannoch"] {
        assert_eq!(
            text.matches(&format!("  {faction} speak [")).count(),
            2,
            "{text}"
        );
        assert!(text.contains(&format!("says \"{faction} holds the line.\"")), "{text}");
    }
    assert!(text.ends_with("status: Completed; 2 of 2 ticks closed\n"), "{text}");
    assert_eq!(responder.served.load(Ordering::SeqCst), 4);

    // The state root holds each store the run wrote.
    for file in ["world.cc", "controller-work.cc", "runs.cc", "calls.cc"] {
        assert!(state.path().join(file).is_file(), "{file} was not written");
    }

    // A second run is the next in the world and carries on from its clock.
    let again = run(state.path(), &responder, "1", "1000000");
    assert!(again.status.success(), "{}", stderr(&again));
    let text = stdout(&again);
    assert!(text.starts_with("run 2: 1 ticks of 60 minutes"), "{text}");
    assert!(text.contains("tick 2 closed: 2 cells, 2 acts, 0 refused\n"), "{text}");
}

#[test]
fn a_cap_the_first_call_could_pass_ends_the_run_with_nothing_spent() {
    let state = tempfile::tempdir().unwrap();
    assert!(genesis(state.path()).status.success());
    let responder = Responder::start();
    let output = run(state.path(), &responder, "3", "1000");
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("tick 0 open: "), "{text}");
    assert!(text.contains("tokens: 0 this tick, 0 spent, 1000 remaining"), "{text}");
    assert!(text.ends_with("status: BudgetExhausted; 0 of 3 ticks closed\n"), "{text}");
    assert_eq!(responder.served.load(Ordering::SeqCst), 0, "no call was made");
}

#[test]
fn the_command_refuses_what_it_cannot_run_and_never_echoes_a_value() {
    let state = tempfile::tempdir().unwrap();
    let responder = Responder::start();

    // No world yet.
    let output = run(state.path(), &responder, "1", "1000");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("run genesis first"), "{}", stderr(&output));

    assert!(genesis(state.path()).status.success());
    // A second genesis does not overwrite the first.
    assert!(!genesis(state.path()).status.success());

    let secret = "sk-not-a-real-key-0000";
    let key = state.path().join(secret);
    let cases: [(Vec<&str>, &str); 6] = [
        (vec![], "usage"),
        (vec!["fly"], "unknown command"),
        (
            vec!["run", "--state", state.path().to_str().unwrap(), "--ticks", "0"],
            "--token-cap is required",
        ),
        (
            vec![
                "run", "--state", state.path().to_str().unwrap(), "--ticks", "many",
                "--token-cap", "1", "--tick-span-minutes", "60", "--model", "local/x",
                "--base-url", "http://127.0.0.1:1",
            ],
            "--ticks is not a positive integer",
        ),
        (
            // A hosted model needs its key file; a missing one is named by flag.
            vec![
                "run", "--state", state.path().to_str().unwrap(), "--ticks", "1",
                "--token-cap", "1", "--tick-span-minutes", "60", "--model", "hosted/x",
                "--base-url", "https://api.example.test", "--key-file", key.to_str().unwrap(),
            ],
            "the inference lane cannot open",
        ),
        (
            vec![
                "run", "--state", state.path().to_str().unwrap(), "--ticks", "1",
                "--token-cap", "1", "--tick-span-minutes", "60", "--model", "local/x",
                "--base-url", "http://127.0.0.1:1", "--key-file", key.to_str().unwrap(),
            ],
            "a local model takes no --key-file",
        ),
    ];
    for (arguments, expected) in cases {
        let output = verse(&arguments);
        assert!(!output.status.success(), "{arguments:?} succeeded");
        let error = stderr(&output);
        assert!(error.contains(expected), "{arguments:?}: {error}");
        assert!(!error.contains(secret), "{arguments:?} echoed a path: {error}");
        assert!(!error.contains("many"), "{arguments:?} echoed a value: {error}");
    }
}
