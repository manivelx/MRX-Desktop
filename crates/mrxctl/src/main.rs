use anyhow::{Context, Result};
use mrx_core::{Request, RequestEnvelope, Response, IPC_PROTOCOL_VERSION};
use std::{
    io::{BufRead, BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn socket() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(format!(
                "/tmp/mrx-{}",
                std::env::var("USER").unwrap_or_else(|_| "user".into())
            ))
        })
        .join("mrx.sock")
}

fn envelope(request: Request) -> RequestEnvelope {
    RequestEnvelope {
        protocol_version: IPC_PROTOCOL_VERSION,
        request_id: NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed),
        request,
    }
}

fn call(request: Request) -> Result<Response> {
    let envelope = envelope(request);
    let mut stream = UnixStream::connect(socket()).context("MRX WM is not running in this session")?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    writeln!(stream, "{}", serde_json::to_string(&envelope)?)?;

    let mut line = String::new();
    let bytes = BufReader::new(stream).take(65_537).read_line(&mut line)?;
    anyhow::ensure!(
        bytes > 0 && bytes <= 65_536 && line.ends_with('\n'),
        "MRX WM returned an empty, oversized, or unterminated IPC response"
    );
    let response: Response = serde_json::from_str(&line).context("invalid MRX WM IPC response")?;
    anyhow::ensure!(
        response.protocol_version == IPC_PROTOCOL_VERSION,
        "MRX WM protocol mismatch: expected {}, received {}",
        IPC_PROTOCOL_VERSION,
        response.protocol_version
    );
    anyhow::ensure!(
        response.request_id == Some(envelope.request_id),
        "MRX WM response ID mismatch: expected {}, received {:?}",
        envelope.request_id,
        response.request_id
    );
    Ok(response)
}

/// Stream snapshots until the WM exits or the subscription is closed.
/// Each event is validated and emitted as one newline-delimited JSON snapshot
/// so Quickshell can consume it with SplitParser.
fn subscribe() -> Result<()> {
    let envelope = envelope(Request::Subscribe);
    let mut stream = UnixStream::connect(socket()).context("MRX WM is not running in this session")?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    writeln!(stream, "{}", serde_json::to_string(&envelope)?)?;

    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        let bytes = reader.by_ref().take(65_537).read_line(&mut line)?;
        if bytes == 0 {
            return Ok(());
        }
        anyhow::ensure!(
            bytes <= 65_536 && line.ends_with('\n'),
            "MRX WM emitted an empty, oversized, or unterminated subscription event"
        );
        let response: Response =
            serde_json::from_str(&line).context("invalid MRX WM subscription event")?;
        anyhow::ensure!(
            response.protocol_version == IPC_PROTOCOL_VERSION,
            "MRX WM protocol mismatch: expected {}, received {}",
            IPC_PROTOCOL_VERSION,
            response.protocol_version
        );
        anyhow::ensure!(
            response.request_id == Some(envelope.request_id),
            "MRX WM subscription ID mismatch: expected {}, received {:?}",
            envelope.request_id,
            response.request_id
        );
        anyhow::ensure!(response.ok, "MRX WM subscription failed: {}", response.message);
        if let Some(mut snapshot) = response.snapshot {
            snapshot.sequence=response.sequence;
            println!("{}", serde_json::to_string(&snapshot)?);
            std::io::stdout().flush()?;
        }
    }
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());
    if command == "subscribe" {
        return subscribe();
    }

    let request = match command.as_str() {
        "snapshot" => Request::Snapshot,
        "ping" => Request::Ping,
        "health" => Request::Health,
        "shutdown" => Request::Shutdown,
        "operation" => Request::GetOperation {
            id: args.next().context("operation ID required")?.parse()?,
        },
        "workspace" => {
            let value = args.next().unwrap_or_else(|| "1".into());
            if value == "next" || value == "previous" {
                Request::SwitchRelative {
                    direction: if value == "next" { "next" } else { "previous" }.into(),
                }
            } else {
                let id = value.parse()?;
                if let Some(monitor) = args.next() {
                    Request::SwitchWorkspaceOnMonitor { monitor, id }
                } else {
                    Request::SwitchWorkspace { id }
                }
            }
        }
        "focus" => Request::FocusWindow {
            id: args.next().context("window XID required")?.parse()?,
        },
        "close" => Request::CloseWindow {
            id: args.next().context("window XID required")?.parse()?,
        },
        "move" => Request::MoveWindow {
            id: args.next().context("window XID required")?.parse()?,
            workspace: args.next().context("workspace required")?.parse()?,
        },
        "move-monitor" => Request::MoveWindowToMonitor {
            id: args.next().context("window XID required")?.parse()?,
            monitor: args.next().context("monitor name required")?,
            workspace: args.next().context("workspace required")?.parse()?,
        },
        "set" => Request::SetSetting {
            key: args.next().context("setting key required")?,
            value: serde_json::from_str(&args.next().context("JSON value required")?)?,
        },
        "reload" => Request::ReloadSettings,
        "undo" => Request::UndoSettings,
        "reset" => Request::ResetSettings,
        "profile" => {
            let action=args.next().context("profile action required: import or export")?;
            let name=args.next().context("profile name required")?;
            match action.as_str() {
                "export" => Request::ExportSettingsProfile{name},
                "import" => Request::ImportSettingsProfile{name},
                _ => anyhow::bail!("profile action must be 'import' or 'export'"),
            }
        },
        _ => {
            println!(
                "mrxctl snapshot|subscribe|ping|health|shutdown|operation <id>|workspace <1-9|next|previous> [monitor]|focus <xid>|close <xid>|move <xid> <workspace>|move-monitor <xid> <monitor> <workspace>|set <key> <json-value>|reload|undo|reset|profile import|export <name>"
            );
            return Ok(());
        }
    };

    let response = call(request)?;
    if let Some(snapshot) = response.snapshot {
        println!("{}", serde_json::to_string(&snapshot)?);
    } else {
        println!("{}", serde_json::to_string(&response)?);
    }
    if !response.ok {
        std::process::exit(1);
    }
    Ok(())
}
