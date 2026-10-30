//! Newline JSON command client. Credentials remain in memory, never in stdout.
use anyhow::{Context, Result, ensure};
use border_net::*;
use reqwest::blocking::Client;
use std::{
    io::{self, BufRead},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() == 3,
        "usage: border-peer http://127.0.0.1:8090 NAME (join code via BORDER_HOST_JOIN_CODE)"
    );
    let url = args[1].trim_end_matches('/').to_string();
    let client = Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()?;
    let joined: JoinResponse = client
        .post(format!("{url}/join"))
        .json(&JoinRequest {
            protocol_version: PROTOCOL_VERSION,
            join_code: std::env::var("BORDER_HOST_JOIN_CODE")
                .context("set BORDER_HOST_JOIN_CODE")?,
            display_name: args[2].clone(),
        })
        .send()?
        .error_for_status()?
        .json()?;
    println!(
        "{}",
        serde_json::json!({"peer_id":joined.peer_id,"session_id":joined.session_id,"snapshot":joined.snapshot})
    );
    let alive = Arc::new(AtomicBool::new(true));
    let running = alive.clone();
    let beat_client = client.clone();
    let beat_url = url.clone();
    let token = joined.peer_token.clone();
    let session = SessionRequest {
        protocol_version: PROTOCOL_VERSION,
        session_id: joined.session_id.clone(),
    };
    let beat_session = session.clone();
    let controls = Arc::new(Mutex::new(([0f32; 2], false)));
    let control_input = controls.clone();
    let heartbeat = std::thread::spawn(move || {
        let mut frame = 0;
        while running.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(100));
            frame += 1;
            let (movement, ready_held) = *control_input.lock().unwrap();
            let _ = beat_client
                .post(format!("{beat_url}/input"))
                .bearer_auth(&token)
                .json(&InputFrame {
                    protocol_version: PROTOCOL_VERSION,
                    session_id: beat_session.session_id.clone(),
                    frame,
                    movement,
                    yaw: std::f32::consts::PI,
                    jog: false,
                    ready_held,
                })
                .send();
        }
    });
    let mut sequence = joined.next_sequence;
    let mut cursor = joined.snapshot.latest_event_sequence;
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(value) = line.strip_prefix("/move ") {
            let parts: Result<Vec<f32>, _> = value.split_whitespace().map(str::parse).collect();
            if let Ok(parts) = parts
                && parts.len() == 2
                && parts.iter().all(|v| v.is_finite() && v.abs() <= 1.)
            {
                controls.lock().unwrap().0 = [parts[0], parts[1]];
                println!("{}", serde_json::json!({"input":"moving"}));
            } else {
                eprintln!("usage: /move X Z (components -1 to 1)");
            }
            continue;
        }
        if ["/stop", "/hold", "/release"].contains(&line.trim()) {
            let mut state = controls.lock().unwrap();
            match line.trim() {
                "/hold" => state.1 = true,
                "/release" => state.1 = false,
                _ => state.0 = [0., 0.],
            };
            println!("{}", serde_json::json!({"input":"updated"}));
            continue;
        }
        if line.trim() == "/snapshot" {
            let snapshot: WorldSnapshot = client
                .get(format!("{url}/snapshot"))
                .bearer_auth(&joined.peer_token)
                .send()?
                .error_for_status()?
                .json()?;
            cursor = snapshot.latest_event_sequence;
            println!("{}", serde_json::to_string(&snapshot)?);
            continue;
        }
        if line.trim() == "/events" {
            let response = client
                .get(format!("{url}/events?after={cursor}"))
                .bearer_auth(&joined.peer_token)
                .send()?;
            if response.status().is_success() {
                let replay: Replay = response.json()?;
                cursor = replay.latest_sequence;
                println!("{}", serde_json::to_string(&replay)?);
            } else {
                eprintln!("{}; use /snapshot to resynchronize", response.text()?);
            }
            continue;
        }
        let command: ClientCommand = match serde_json::from_str(&line) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{e}");
                continue;
            }
        };
        let envelope = CommandEnvelope {
            protocol_version: PROTOCOL_VERSION,
            session_id: joined.session_id.clone(),
            request_id: format!("{}-{sequence}", joined.peer_id),
            sequence,
            expected_revision: None,
            command,
        };
        let send = || {
            client
                .post(format!("{url}/command"))
                .bearer_auth(&joined.peer_token)
                .json(&envelope)
                .send()
        };
        let response = match send() {
            Ok(r) => r,
            Err(_) => send()?,
        };
        if response.status().is_success() {
            let receipt: CommandReceipt = response.json()?;
            sequence += 1;
            println!("{}", serde_json::to_string(&receipt)?);
        } else {
            eprintln!("{}", response.text()?);
        }
    }
    alive.store(false, Ordering::Relaxed);
    let _ = heartbeat.join();
    client
        .post(format!("{url}/leave"))
        .bearer_auth(&joined.peer_token)
        .json(&session)
        .send()?
        .error_for_status()?;
    Ok(())
}
