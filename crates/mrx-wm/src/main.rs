use anyhow::{Context, Result};
use mrx_core::{HealthReport, OperationError, OperationResult, OperationStatus, Request, RequestEnvelope, Response, Settings, Snapshot, WindowInfo, WorkspaceInfo, MonitorInfo, IPC_PROTOCOL_VERSION};
use std::{collections::BTreeMap, io::{BufRead, BufReader, Read, Write}, os::unix::{fs::{FileTypeExt, PermissionsExt}, net::{UnixListener, UnixStream}}, path::PathBuf, sync::{atomic::{AtomicUsize, Ordering}, Arc, Mutex, mpsc}, thread, time::{Duration,Instant}};
use tracing::{info, warn};
use x11rb::{connection::{Connection, RequestConnection}, properties::{WmHints, WmSizeHints}, protocol::{randr::{self, ConnectionExt as RandrConnectionExt, NotifyMask}, xproto::{self, ConnectionExt as XprotoConnectionExt, AtomEnum, ChangeWindowAttributesAux, ConfigureWindowAux, EventMask, GrabMode, InputFocus, ModMask, Window}, Event}, rust_connection::RustConnection, wrapper::ConnectionExt as WrapperConnectionExt, CURRENT_TIME};

#[derive(Default)] struct State { active:u32, clients:BTreeMap<u32,WindowInfo>, client_order:Vec<u32>, settings:Settings, monitors:Vec<MonitorInfo>, size_hints:BTreeMap<u32,WmSizeHints>, restore_geometry:BTreeMap<u32,(mrx_core::layout::Rect,bool)>, active_by_monitor:BTreeMap<String,u32>, focused_monitor:String, docks:BTreeMap<u32,DockStrut>, root_width:u32, root_height:u32, operations:BTreeMap<u64,OperationResult>, next_operation_id:u64, settings_history:Vec<Settings>, started_at:Option<Instant>, active_subscriptions:usize, subscription_events_sent:u64, subscription_bytes_sent:u64, subscription_disconnects:u64, subscription_slow_clients_dropped:u64, subscription_write_timeouts:u64, subscription_max_write_latency_ms:u64, snapshot_sequence:u64 }
#[derive(Clone, Copy)]
struct WorkspaceAtoms {
    current: u32,
    wm_state: u32,
    net_wm_state: u32,
    net_wm_state_hidden: u32,
    net_active_window: u32,
}

#[derive(Debug)] enum Action { Switch(u32), SwitchMonitor(String,u32), Relative(i32), Focus(u32), Close(u32), Move(u32,u32), Set(String,serde_json::Value), Reload, UndoSettings, ResetSettings, ExportProfile(String), ImportProfile(String), Shutdown, Operation(u64,Box<Action>) }
fn runtime_dir()->PathBuf { std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(||PathBuf::from(format!("/tmp/mrx-{}",std::env::var("USER").unwrap_or_else(|_|"user".into())))) }

/// Queue an asynchronous WM operation and return its stable operation ID.
/// Results are retained in a bounded history for polling by IPC clients.
fn enqueue_operation(state:&Arc<Mutex<State>>,tx:&mpsc::Sender<Action>,kind:&str,action:Action)->Response {
    let queued={
        let mut state=state.lock().unwrap();
        let id=state.next_operation_id.max(1);
        state.next_operation_id=id.wrapping_add(1).max(1);
        let result=OperationResult{id,kind:kind.into(),status:OperationStatus::Queued,message:"operation queued".into(),error:None};
        state.operations.insert(id,result.clone());
        while state.operations.len()>128 {
            if let Some(oldest)=state.operations.keys().next().copied(){state.operations.remove(&oldest);}else{break;}
        }
        result
    };
    if tx.send(Action::Operation(queued.id,Box::new(action))).is_err() {
        let error=OperationError{code:"event_loop_unavailable".into(),message:"WM event loop is unavailable".into(),retryable:true};
        let mut state=state.lock().unwrap();
        if let Some(result)=state.operations.get_mut(&queued.id) {
            result.status=OperationStatus::Failed;
            result.message=error.message.clone();
            result.error=Some(error);
        }
    }
    let result=state.lock().unwrap().operations.get(&queued.id).cloned().unwrap_or(queued);
    Response::with_operation(result)
}


/// Keep a bounded in-memory undo history; never grow with every setting edit.
fn push_settings_history(state:&mut State,settings:Settings) {
    const MAX_SETTINGS_HISTORY:usize=20;
    state.settings_history.push(settings);
    if state.settings_history.len()>MAX_SETTINGS_HISTORY {
        let excess=state.settings_history.len()-MAX_SETTINGS_HISTORY;
        state.settings_history.drain(0..excess);
    }
}

fn finish_operation(state:&mut State,id:u64,error:Option<OperationError>) {
    if let Some(operation)=state.operations.get_mut(&id) {
        if let Some(error)=error {
            operation.status=OperationStatus::Failed;
            operation.message=error.message.clone();
            operation.error=Some(error);
        } else {
            operation.status=OperationStatus::Succeeded;
            operation.message="operation completed".into();
            operation.error=None;
        }
    }
}

fn active_workspace_for(s: &State, monitor: &str) -> u32 {
    s.active_by_monitor.get(monitor).copied().unwrap_or(1).clamp(1, s.settings.workspace.count as u32)
}

fn focused_monitor_name(s: &State) -> String {
    if s.monitors.iter().any(|m| m.name == s.focused_monitor) {
        return s.focused_monitor.clone();
    }
    s.monitors.iter().find(|m| m.focused).or_else(|| s.monitors.first())
        .map(|m| m.name.clone()).unwrap_or_default()
}

fn set_focused_monitor(s: &mut State, name: &str) {
    if !s.monitors.iter().any(|m| m.name == name) { return; }
    s.focused_monitor = name.to_owned();
    for monitor in &mut s.monitors { monitor.focused = monitor.name == name; }
    s.active = active_workspace_for(s, name);
}

fn snapshot(s:&State)->Snapshot {
    let focused = focused_monitor_name(s);
    let active = active_workspace_for(s, &focused);
    let mut workspaces = Vec::new();
    for monitor in &s.monitors {
        let monitor_active = active_workspace_for(s, &monitor.name);
        for id in 1..=s.settings.workspace.count as u32 {
            workspaces.push(WorkspaceInfo {
                id,
                monitor: monitor.name.clone(),
                name: s.settings.workspace.names.get((id-1) as usize).cloned().unwrap_or_else(||id.to_string()),
                active: id == monitor_active,
                windows: s.clients.values().filter(|w|w.workspace==id && w.monitor==monitor.name && !w.dock).count(),
            });
        }
    }
    let mut monitors = s.monitors.clone();
    for monitor in &mut monitors {
        monitor.active_workspace = active_workspace_for(s, &monitor.name);
        monitor.focused = monitor.name == focused;
    }
    Snapshot{protocol_version:mrx_core::IPC_PROTOCOL_VERSION,compositor:"mrx-x11".into(),active_workspace:active,workspaces,windows:s.clients.values().cloned().collect(),settings:s.settings.clone(),monitors}
}
fn monitor_for_geometry(monitors: &[MonitorInfo], x: i32, y: i32, width: u32, height: u32) -> String {
    let center_x = x + (width / 2) as i32;
    let center_y = y + (height / 2) as i32;
    monitors.iter()
        .find(|m| center_x >= m.x && center_x < m.x + m.width as i32
            && center_y >= m.y && center_y < m.y + m.height as i32)
        .or_else(|| monitors.iter().find(|m| m.focused))
        .or_else(|| monitors.first())
        .map(|m| m.name.clone())
        .unwrap_or_default()
}

struct SubscriptionGuard { state:Arc<Mutex<State>> }
impl Drop for SubscriptionGuard {
    fn drop(&mut self) {
        let mut state=self.state.lock().unwrap();
        state.active_subscriptions=state.active_subscriptions.saturating_sub(1);
        state.subscription_disconnects=state.subscription_disconnects.saturating_add(1);
    }
}

fn record_subscription_write(state:&mut State,bytes:usize,elapsed:Duration,error:Option<std::io::ErrorKind>) {
    let latency=elapsed.as_millis().min(u64::MAX as u128) as u64;
    state.subscription_max_write_latency_ms=state.subscription_max_write_latency_ms.max(latency);
    match error {
        None=>{
            state.subscription_events_sent=state.subscription_events_sent.saturating_add(1);
            state.subscription_bytes_sent=state.subscription_bytes_sent.saturating_add(bytes as u64);
        }
        Some(kind)=>{
            if matches!(kind,std::io::ErrorKind::TimedOut|std::io::ErrorKind::WouldBlock) {
                state.subscription_write_timeouts=state.subscription_write_timeouts.saturating_add(1);
                state.subscription_slow_clients_dropped=state.subscription_slow_clients_dropped.saturating_add(1);
            }
        }
    }
}

fn stream_ipc_snapshots(
    mut stream: UnixStream,
    state: &Arc<Mutex<State>>,
    request_id: u64,
) -> Result<()> {
    // A subscription owns one bounded IPC worker. Slow readers are disconnected
    // on write timeout; event sequence numbers make coalescing/gaps observable.
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let _subscription={
        let mut state_guard=state.lock().unwrap();
        state_guard.active_subscriptions=state_guard.active_subscriptions.saturating_add(1);
        SubscriptionGuard{state:Arc::clone(state)}
    };
    let mut probe=stream.try_clone()?;
    probe.set_read_timeout(Some(Duration::from_millis(100)))?;
    let mut previous=String::new();
    loop {
        let (current,encoded,sequence)={
            let mut state=state.lock().unwrap();
            let current=snapshot(&state);
            let encoded=serde_json::to_string(&current)?;
            if encoded==previous { (None,encoded,None) }
            else {
                state.snapshot_sequence=state.snapshot_sequence.saturating_add(1);
                (Some(current),encoded,Some(state.snapshot_sequence))
            }
        };
        if let (Some(current),Some(sequence))=(current,sequence) {
            let mut event=Response::ok("snapshot event");
            event.request_id=Some(request_id);
            event.snapshot=Some(current);
            event.sequence=Some(sequence);
            let mut frame=serde_json::to_vec(&event)?;
            frame.push(b'\\n');
            let started=Instant::now();
            let result=stream.write_all(&frame).and_then(|_|stream.flush());
            let elapsed=started.elapsed();
            let error=result.as_ref().err().map(|error|error.kind());
            record_subscription_write(&mut state.lock().unwrap(),frame.len(),elapsed,error);
            if let Err(error)=result { return Err(error.into()); }
            previous=encoded;
        }

        // Probe for disconnects even when no state changes occur.
        let mut incoming=[0u8;1];
        match probe.read(&mut incoming) {
            Ok(0)=>return Ok(()),
            Ok(_)=>anyhow::bail!("unexpected client data on snapshot subscription"),
            Err(error) if matches!(error.kind(),std::io::ErrorKind::WouldBlock|std::io::ErrorKind::TimedOut)=>{},
            Err(error)=>return Err(error.into()),
        }
    }
}

fn handle_ipc_connection(mut stream: UnixStream, state: &Arc<Mutex<State>>, tx: &mpsc::Sender<Action>, active_clients:&Arc<AtomicUsize>) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;

    let mut line = String::new();
    let mut reader = BufReader::new(stream.try_clone()?);
    let bytes = reader.by_ref().take(65_537).read_line(&mut line)?;
    if bytes == 0 {
        anyhow::bail!("empty IPC request");
    }
    if bytes > 65_536 || !line.ends_with('\n') {
        anyhow::bail!("IPC request exceeds 64 KiB or is not newline-terminated");
    }

    let request: Result<RequestEnvelope, _> = serde_json::from_str(&line);
    if let Ok(envelope) = request.as_ref() {
        if envelope.validate().is_ok() && matches!(&envelope.request, Request::Subscribe) {
            return stream_ipc_snapshots(stream, state, envelope.request_id);
        }
    }
    let response = match request {
        Err(error) => Response::err_code("invalid_request",format!("invalid request envelope: {error}"),false),
        Ok(envelope) => {
            let request_id = envelope.request_id;
            let mut response = if let Err(error) = envelope.validate() {
                Response::err_code("unsupported_protocol_version",error.to_string(),false)
            } else {
                match envelope.request {
                    Request::Ping => Response::ok("pong"),
                    Request::GetOperation { id } => {
                        match state.lock().unwrap().operations.get(&id).cloned() {
                            Some(operation) => Response::with_operation(operation),
                            None => Response::err_code("operation_not_found",format!("operation {id} was not found or has expired"),false),
                        }
                    },
                    Request::UndoSettings => enqueue_operation(state,tx,"undo_settings",Action::UndoSettings),
                    Request::ResetSettings => enqueue_operation(state,tx,"reset_settings",Action::ResetSettings),
                    Request::ExportSettingsProfile{name} => enqueue_operation(state,tx,"export_settings_profile",Action::ExportProfile(name)),
                    Request::ImportSettingsProfile{name} => enqueue_operation(state,tx,"import_settings_profile",Action::ImportProfile(name)),
                    Request::Health => {
                        let state=state.lock().unwrap();
                        let operations_queued=state.operations.values().filter(|op|op.status==OperationStatus::Queued).count();
                        let operations_succeeded=state.operations.values().filter(|op|op.status==OperationStatus::Succeeded).count();
                        let operations_failed=state.operations.values().filter(|op|op.status==OperationStatus::Failed).count();
                        Response::with_health(HealthReport{
                            pid:std::process::id(),
                            uptime_ms:state.started_at.map(|started|started.elapsed().as_millis().min(u64::MAX as u128) as u64).unwrap_or(0),
                            managed_windows:state.clients.len(),
                            monitor_count:state.monitors.len(),
                            active_ipc_clients:active_clients.load(Ordering::Relaxed),
                            active_subscriptions:state.active_subscriptions,
                            subscription_events_sent:state.subscription_events_sent,
                            subscription_bytes_sent:state.subscription_bytes_sent,
                            subscription_disconnects:state.subscription_disconnects,
                            subscription_slow_clients_dropped:state.subscription_slow_clients_dropped,
                            subscription_write_timeouts:state.subscription_write_timeouts,
                            subscription_max_write_latency_ms:state.subscription_max_write_latency_ms,
                            operations_queued,operations_succeeded,operations_failed,
                        })
                    },
                    Request::Shutdown => enqueue_operation(state,tx,"shutdown",Action::Shutdown),
                    Request::Snapshot => {
                        let mut response = Response::ok("snapshot");
                        response.snapshot = Some(snapshot(&state.lock().unwrap()));
                        response
                    }
                    Request::Subscribe => Response::ok("subscription started"),
                    Request::ReloadSettings => {
                        enqueue_operation(state,tx,"reload_settings",Action::Reload)
                    }
                    Request::SetSetting { key, value } => {
                        enqueue_operation(state,tx,"set_setting",Action::Set(key,value))
                    }
                    Request::SwitchWorkspace { id } => {
                        enqueue_operation(state,tx,"switch_workspace",Action::Switch(id))
                    }
                    Request::SwitchWorkspaceOnMonitor { monitor, id } => {
                        enqueue_operation(state,tx,"switch_monitor_workspace",Action::SwitchMonitor(monitor,id))
                    }
                    Request::SwitchRelative { direction } => {
                        if direction!="next" && direction!="previous" {
                            Response::err_code("invalid_direction","direction must be 'next' or 'previous'",false)
                        } else {
                            let delta=if direction=="next" {1}else{-1};
                            enqueue_operation(state,tx,"switch_relative",Action::Relative(delta))
                        }
                    }
                    Request::FocusWindow { id } => {
                        enqueue_operation(state,tx,"focus_window",Action::Focus(id))
                    }
                    Request::CloseWindow { id } => {
                        enqueue_operation(state,tx,"close_window",Action::Close(id))
                    }
                    Request::MoveWindow { id, workspace } => {
                        enqueue_operation(state,tx,"move_window",Action::Move(id,workspace))
                    }
                }
            };
            response.protocol_version = IPC_PROTOCOL_VERSION;
            response.request_id = Some(request_id);
            response
        }
    };

    writeln!(stream, "{}", serde_json::to_string(&response)?)?;
    Ok(())
}

fn serve_ipc(state: Arc<Mutex<State>>, tx: mpsc::Sender<Action>) -> Result<()> {
    let dir = runtime_dir();
    std::fs::create_dir_all(&dir)?;
    // Only chmod the private fallback directory; never change the caller's
    // existing XDG_RUNTIME_DIR permissions.
    if std::env::var_os("XDG_RUNTIME_DIR").is_none() {
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }

    let path = dir.join("mrx.sock");
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_socket() => std::fs::remove_file(&path)?,
        Ok(_) => anyhow::bail!("refusing to replace non-socket IPC path: {}", path.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    info!(socket = %path.display(), "IPC listening");

    // Bound concurrent clients while keeping slow IPC clients from blocking
    // unrelated requests. The cap prevents unbounded local thread creation.
    const MAX_IPC_CLIENTS: usize = 32;
    let active_clients = Arc::new(AtomicUsize::new(0));
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let mut current = active_clients.load(Ordering::Relaxed);
                loop {
                    if current >= MAX_IPC_CLIENTS {
                        warn!("IPC client limit reached; rejecting connection");
                        drop(stream);
                        break;
                    }
                    match active_clients.compare_exchange_weak(
                        current,
                        current + 1,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => {
                            let client_state = Arc::clone(&state);
                            let client_tx = tx.clone();
                            let client_count = Arc::clone(&active_clients);
                            let spawn = thread::Builder::new()
                                .name("mrx-ipc-client".into())
                                .spawn(move || {
                                    if let Err(error) = handle_ipc_connection(stream, &client_state, &client_tx, &client_count) {
                                        warn!(error = %error, "IPC client request failed");
                                    }
                                    client_count.fetch_sub(1, Ordering::AcqRel);
                                });
                            if let Err(error) = spawn {
                                active_clients.fetch_sub(1, Ordering::AcqRel);
                                warn!(error = %error, "failed to start IPC worker");
                            }
                            break;
                        }
                        Err(observed) => current = observed,
                    }
                }
            }
            Err(error) => warn!(error = %error, "IPC accept failed"),
        }
    }
    Ok(())
}
fn intern(conn:&RustConnection,name:&str)->Result<u32>{Ok(conn.intern_atom(false,name.as_bytes())?.reply()?.atom)}

/// Query RandR 1.5 monitor objects. Fall back to the X screen geometry when
/// RandR is unavailable or the server does not expose monitor objects.
fn discard_synthetic_screen_monitor(monitors: &mut Vec<MonitorInfo>) {
    // Some RandR servers expose the root framebuffer as a synthetic
    // "screen" monitor alongside explicitly configured output monitors.
    // Drop that overlapping fallback once real monitor objects are present.
    if monitors.iter().any(|monitor| monitor.name != "screen") {
        monitors.retain(|monitor| monitor.name != "screen");
    }
}

fn discover_monitors(conn: &RustConnection, root: Window, width: u16, height: u16) -> Vec<MonitorInfo> {
    let mut monitors = Vec::new();
    // RandR 1.5 exposes monitor objects independently from physical outputs.
    // Probe the server version first: some X servers advertise the extension
    // but do not implement GetMonitors (notably minimal/virtual servers).
    let randr_available = conn.extension_information(randr::X11_EXTENSION_NAME)
        .ok().flatten().is_some();
    let randr_version = if randr_available {
        conn.randr_query_version(1, 5).ok().and_then(|cookie| cookie.reply().ok())
    } else {
        None
    };
    let supports_monitor_objects = randr_version.as_ref()
        .is_some_and(|version| (version.major_version, version.minor_version) >= (1, 5));
    if supports_monitor_objects {
        match conn.randr_get_monitors(root, false) {
            Ok(cookie) => match cookie.reply() {
            Ok(reply) => {
                for monitor in reply.monitors {
                    if monitor.width == 0 || monitor.height == 0 { continue; }
                    let name = conn.get_atom_name(monitor.name).ok()
                        .and_then(|cookie| cookie.reply().ok())
                        .map(|atom| String::from_utf8_lossy(&atom.name).into_owned())
                        .unwrap_or_else(|| format!("monitor-{}", monitor.name));
                    monitors.push(MonitorInfo {
                        name, x: monitor.x as i32, y: monitor.y as i32,
                        width: monitor.width as u32, height: monitor.height as u32,
                        focused: monitor.primary, active_workspace: 1,
                    });
                }
            }
            Err(error) => warn!(%error, "RandR GetMonitors reply failed; using active CRTC outputs"),
            },
            Err(error) => warn!(%error, "RandR GetMonitors request failed; using active CRTC outputs"),
        }
    }

    discard_synthetic_screen_monitor(&mut monitors);

    // Compatibility fallback: construct monitor rectangles from active CRTCs.
    // This still supports legacy RandR 1.2 servers and Xvfb configurations
    // where a root-screen has several attached output regions.
    if monitors.is_empty() && randr_available {
        if let Ok(resources_cookie) = conn.randr_get_screen_resources_current(root) {
          if let Ok(resources) = resources_cookie.reply() {
            let mut crtcs = Vec::new();
            for crtc_id in resources.crtcs {
                let Ok(info_cookie) = conn.randr_get_crtc_info(crtc_id, resources.config_timestamp) else { continue; };
                let Ok(info) = info_cookie.reply() else { continue; };
                if info.mode == 0 || info.width == 0 || info.height == 0 { continue; }
                let names = info.outputs.iter().filter_map(|output_id| {
                    conn.randr_get_output_info(*output_id, resources.config_timestamp).ok()
                        .and_then(|cookie| cookie.reply().ok())
                        .map(|output| String::from_utf8_lossy(&output.name).into_owned())
                }).filter(|name| !name.is_empty()).collect::<Vec<_>>();
                let name = if names.is_empty() {
                    format!("CRTC-{crtc_id}")
                } else {
                    names.join("+")
                };
                crtcs.push(MonitorInfo {
                    name,
                    x: info.x as i32,
                    y: info.y as i32,
                    width: info.width as u32,
                    height: info.height as u32,
                    focused: false, active_workspace: 1,
                });
            }
            crtcs.sort_by(|a, b| (a.x, a.y, &a.name).cmp(&(b.x, b.y, &b.name)));
            monitors = crtcs;
          }
        }
    }
    if monitors.is_empty() {
        monitors.push(MonitorInfo {
            name: "screen-0".into(), x: 0, y: 0,
            width: width as u32, height: height as u32, focused: true, active_workspace: 1,
        });
    } else if !monitors.iter().any(|monitor| monitor.focused) {
        monitors[0].focused = true;
    }
    monitors
}

/// Request a polite ICCCM close. Only force-kill clients that do not advertise
/// WM_DELETE_WINDOW; clients that support it must be allowed to save/confirm.
fn request_close(conn: &RustConnection, window: Window, wm_protocols: u32, wm_delete: u32) -> Result<()> {
    let reply = conn.get_property(false, window, wm_protocols, AtomEnum::ATOM, 0, 64)?.reply()?;
    let supports_delete = reply.value32().map(|mut atoms| atoms.any(|atom| atom == wm_delete)).unwrap_or(false);
    if supports_delete {
        let event = xproto::ClientMessageEvent::new(
            32, window, wm_protocols,
            [wm_delete, CURRENT_TIME, 0, 0, 0],
        );
        conn.send_event(false, window, EventMask::NO_EVENT, event)?;
    } else {
        conn.kill_client(window)?;
    }
    Ok(())
}

/// Read WM_PROTOCOLS once and test whether a client implements a protocol.
fn read_size_hints(conn: &RustConnection, window: Window) -> Option<WmSizeHints> {
    WmSizeHints::get_normal_hints(conn, window).ok()
        .and_then(|cookie| cookie.reply().ok())
        .flatten()
}

fn has_fixed_size(hints: Option<&WmSizeHints>) -> bool {
    hints.and_then(|h| h.min_size.zip(h.max_size))
        .is_some_and(|(min, max)| min == max && min.0 > 0 && min.1 > 0)
}

/// Fit one axis to ICCCM min/max/base/increment constraints without
/// introducing zero-sized client geometry.
fn fit_size_axis(
    available: u32,
    minimum: Option<i32>,
    maximum: Option<i32>,
    base: Option<i32>,
    increment: Option<i32>,
) -> u32 {
    let min = minimum.filter(|v| *v > 0).unwrap_or(1) as u32;
    let max = maximum.filter(|v| *v > 0).map(|v| (v as u32).max(min)).unwrap_or(u32::MAX);
    let target = available.max(min).min(max);
    let step = increment.filter(|v| *v > 0).unwrap_or(1) as u32;
    let origin = base.filter(|v| *v >= 0).unwrap_or(min as i32) as u32;
    let quantized = if target >= origin {
        origin.saturating_add(((target - origin) / step).saturating_mul(step))
    } else {
        origin
    };
    quantized.clamp(min, max)
}

/// Apply ICCCM min/max/base/increment and aspect hints to a tiled cell.
/// The resulting client is centered in the original cell after constraints.
fn constrain_rect(rect: mrx_core::layout::Rect, hints: Option<&WmSizeHints>) -> mrx_core::layout::Rect {
    let Some(hints) = hints else { return rect; };
    let min = hints.min_size.unwrap_or((1, 1));
    let max = hints.max_size.unwrap_or((0, 0));
    let base = hints.base_size.unwrap_or(min);
    let increment = hints.size_increment.unwrap_or((1, 1));
    let min_w = min.0.max(1) as u32;
    let min_h = min.1.max(1) as u32;
    let mut width = fit_size_axis(rect.width, Some(min.0), (max.0 > 0).then_some(max.0), Some(base.0), Some(increment.0));
    let mut height = fit_size_axis(rect.height, Some(min.1), (max.1 > 0).then_some(max.1), Some(base.1), Some(increment.1));

    if let Some((minimum_aspect, maximum_aspect)) = hints.aspect {
        if minimum_aspect.numerator > 0 && minimum_aspect.denominator > 0
            && (width as i64) * (minimum_aspect.denominator as i64)
                < (height as i64) * (minimum_aspect.numerator as i64)
        {
            let candidate = ((width as i64) * (minimum_aspect.denominator as i64)
                / (minimum_aspect.numerator as i64)).max(1) as u32;
            if candidate >= min_h { height = candidate.min(height); }
            else {
                let candidate = ((height as i64) * (minimum_aspect.numerator as i64)
                    + minimum_aspect.denominator as i64 - 1)
                    / minimum_aspect.denominator as i64;
                width = width.max(candidate as u32);
            }
        }
        if maximum_aspect.numerator > 0 && maximum_aspect.denominator > 0
            && (width as i64) * (maximum_aspect.denominator as i64)
                > (height as i64) * (maximum_aspect.numerator as i64)
        {
            let candidate = ((height as i64) * (maximum_aspect.numerator as i64)
                / (maximum_aspect.denominator as i64)).max(1) as u32;
            if candidate >= min_w { width = candidate.min(width); }
            else {
                let candidate = ((width as i64) * (maximum_aspect.denominator as i64)
                    + maximum_aspect.numerator as i64 - 1)
                    / maximum_aspect.numerator as i64;
                height = height.max(candidate as u32);
            }
        }
    }

    let dx = (rect.width as i64 - width as i64) / 2;
    let dy = (rect.height as i64 - height as i64) / 2;
    let x = (rect.x as i64 + dx).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    let y = (rect.y as i64 + dy).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    mrx_core::layout::Rect { x, y, width, height }
}


/// Compute an EWMH maximize rectangle from the pre-maximize geometry. Each
/// axis is independent so horizontal-only and vertical-only requests restore
/// the untouched axis exactly.
fn maximized_rect(
    original: mrx_core::layout::Rect,
    workarea: mrx_core::layout::Rect,
    horizontal: bool,
    vertical: bool,
) -> mrx_core::layout::Rect {
    mrx_core::layout::Rect {
        x: if horizontal { workarea.x } else { original.x },
        y: if vertical { workarea.y } else { original.y },
        width: if horizontal { workarea.width } else { original.width },
        height: if vertical { workarea.height } else { original.height },
    }
}

fn read_urgent_hint(conn: &RustConnection, window: Window) -> bool {
    WmHints::get(conn, window).ok()
        .and_then(|cookie| cookie.reply().ok())
        .flatten()
        .is_some_and(|hints| hints.urgent)
}

fn supports_protocol(conn: &RustConnection, window: Window, wm_protocols: u32, protocol: u32) -> Result<bool> {
    let reply = conn.get_property(false, window, wm_protocols, AtomEnum::ATOM, 0, 64)?.reply()?;
    Ok(reply.value32().map(|atoms| atoms.into_iter().any(|atom| atom == protocol)).unwrap_or(false))
}

/// Apply the ICCCM input model. Clients with InputHint=false may still request
/// focus through WM_TAKE_FOCUS; never send both mechanisms blindly.
fn focus_client(conn: &RustConnection, window: Window, wm_protocols: u32, wm_take_focus: u32) -> Result<()> {
    let hints = WmHints::get(conn, window)?.reply()?.unwrap_or_default();
    if hints.input.unwrap_or(true) {
        conn.set_input_focus(InputFocus::POINTER_ROOT, window, CURRENT_TIME)?;
    }
    if supports_protocol(conn, window, wm_protocols, wm_take_focus)? {
        let event = xproto::ClientMessageEvent::new(
            32, window, wm_protocols, [wm_take_focus, CURRENT_TIME, 0, 0, 0],
        );
        conn.send_event(false, window, EventMask::NO_EVENT, event)?;
    }
    Ok(())
}


/// Choose the most recently managed visible modal first, then the most recent
/// ordinary client on the focused monitor/workspace.
fn preferred_focus_client(state: &State) -> Option<u32> {
    let monitor=focused_monitor_name(state);
    let active=active_workspace_for(state,&monitor);
    let eligible=|id:&u32| state.clients.get(id).is_some_and(|w|
        !w.dock && !w.minimized && w.monitor==monitor && w.workspace==active);
    state.client_order.iter().rev().find(|id|eligible(id) && state.clients.get(id).is_some_and(|w|w.modal))
        .copied()
        .or_else(||state.client_order.iter().rev().find(|id|eligible(id)).copied())
}

/// Redirect a focus request to the deepest visible modal descendant of the
/// requested client. The bounded walk also protects against malformed cycles.
fn modal_focus_target(state: &State, requested: u32) -> u32 {
    let mut target=requested;
    for _ in 0..16 {
        let Some(parent)=state.clients.get(&target) else { break; };
        let next=state.client_order.iter().rev().find(|id|state.clients.get(id).is_some_and(|w|
            w.modal && !w.minimized && !w.dock && w.transient_for==target
                && w.monitor==parent.monitor && w.workspace==parent.workspace)).copied();
        match next { Some(child) if child!=target=>target=child, _=>break }
    }
    target
}

/// Restore focus to the most recently managed client on the active workspace.
fn focus_most_recent(
    conn: &RustConnection,
    root: Window,
    state: &mut State,
    wm_protocols: u32,
    wm_take_focus: u32,
    net_active_window: u32,
) {
    let candidate = preferred_focus_client(state);
    for window in state.clients.values_mut() {
        window.focused = Some(window.id) == candidate;
    }
    if let Some(id) = candidate {
        if let Err(error) = focus_client(conn, id, wm_protocols, wm_take_focus) {
            warn!(window = id, %error, "failed to restore client focus");
        }
        let _ = conn.change_property32(
            xproto::PropMode::REPLACE, root, net_active_window, AtomEnum::WINDOW, &[id],
        );
    } else {
        let _ = conn.delete_property(root, net_active_window);
    }
}

/// Set the ICCCM WM_STATE property (Withdrawn=0, Normal=1, Iconic=3).
fn set_wm_state(conn: &RustConnection, window: Window, wm_state: u32, state: u32) -> Result<()> {
    conn.change_property32(xproto::PropMode::REPLACE, window, wm_state, wm_state, &[state, 0])?;
    Ok(())
}

/// Add or remove one atom from an EWMH state property without disturbing
/// other state atoms owned by clients or the window manager.
fn set_atom_state(conn: &RustConnection, window: Window, property: u32, atom: u32, enabled: bool) -> Result<()> {
    let reply = conn.get_property(false, window, property, AtomEnum::ATOM, 0, 64)?.reply()?;
    let mut atoms: Vec<u32> = reply.value32().map(|values| values.collect()).unwrap_or_default();
    atoms.retain(|value| *value != atom);
    if enabled { atoms.push(atom); }
    if atoms.is_empty() {
        conn.delete_property(window, property)?;
    } else {
        conn.change_property32(xproto::PropMode::REPLACE, window, property, AtomEnum::ATOM, &atoms)?;
    }
    Ok(())
}


/// Read an existing EWMH atom-list property. Missing or malformed properties
/// are treated as an empty list; the client remains manageable.
fn read_atom_list(conn: &RustConnection, window: Window, property: u32) -> Vec<u32> {
    conn.get_property(false, window, property, AtomEnum::ATOM, 0, 64).ok()
        .and_then(|cookie| cookie.reply().ok())
        .and_then(|reply| reply.value32().map(|values| values.collect()))
        .unwrap_or_default()
}

/// Read the first CARDINAL from an ICCCM/EWMH property when its type matches.
fn read_cardinal(conn: &RustConnection, window: Window, property: u32, property_type: u32) -> Option<u32> {
    conn.get_property(false, window, property, property_type, 0, 1).ok()?
        .reply().ok()?.value32()?.next()
}

/// Publish EWMH client lists with their required ordering semantics.
/// `_NET_CLIENT_LIST` follows initial map order; `_NET_CLIENT_LIST_STACKING`
/// follows the X server's bottom-to-top child stacking order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct DockStrut {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
    left_start_y: u32,
    left_end_y: u32,
    right_start_y: u32,
    right_end_y: u32,
    top_start_x: u32,
    top_end_x: u32,
    bottom_start_x: u32,
    bottom_end_x: u32,
}

fn read_dock_strut(conn: &RustConnection, window: Window, partial_atom: u32, legacy_atom: u32) -> Option<DockStrut> {
    let partial = conn.get_property(false, window, partial_atom, AtomEnum::CARDINAL, 0, 12).ok()?.reply().ok()?;
    if let Some(values) = partial.value32() {
        let v: Vec<u32> = values.collect();
        if v.len() >= 12 {
            return Some(DockStrut {
                left:v[0], right:v[1], top:v[2], bottom:v[3],
                left_start_y:v[4], left_end_y:v[5],
                right_start_y:v[6], right_end_y:v[7],
                top_start_x:v[8], top_end_x:v[9],
                bottom_start_x:v[10], bottom_end_x:v[11],
            });
        }
    }
    let legacy = conn.get_property(false, window, legacy_atom, AtomEnum::CARDINAL, 0, 4).ok()?.reply().ok()?;
    let v: Vec<u32> = legacy.value32()?.collect();
    if v.len() < 4 { return None; }
    Some(DockStrut { left:v[0],right:v[1],top:v[2],bottom:v[3],..Default::default() })
}

fn dock_strut_intersects_monitor(strut: &DockStrut, monitor: &MonitorInfo) -> bool {
    let mx0=monitor.x.max(0) as u32;
    let my0=monitor.y.max(0) as u32;
    let mx1=mx0.saturating_add(monitor.width);
    let my1=my0.saturating_add(monitor.height);
    let x_intersects = |start:u32,end:u32| end==0 || (start < mx1 && end >= mx0);
    let y_intersects = |start:u32,end:u32| end==0 || (start < my1 && end >= my0);
    (strut.left>0 && y_intersects(strut.left_start_y,strut.left_end_y))
        || (strut.right>0 && y_intersects(strut.right_start_y,strut.right_end_y))
        || (strut.top>0 && x_intersects(strut.top_start_x,strut.top_end_x))
        || (strut.bottom>0 && x_intersects(strut.bottom_start_x,strut.bottom_end_x))
}

fn monitor_workarea(
    monitor: &MonitorInfo,
    docks: &BTreeMap<u32,DockStrut>,
    root_width: u32,
    root_height: u32,
) -> mrx_core::layout::Rect {
    let mx0=monitor.x.max(0) as u32;
    let my0=monitor.y.max(0) as u32;
    let mx1=mx0.saturating_add(monitor.width);
    let my1=my0.saturating_add(monitor.height);
    let mut left=0u32; let mut right=0u32; let mut top=0u32; let mut bottom=0u32;
    for strut in docks.values().filter(|s|dock_strut_intersects_monitor(s,monitor)) {
        left=left.max(strut.left.saturating_sub(mx0).min(monitor.width));
        let right_edge=root_width.saturating_sub(strut.right);
        if right_edge < mx1 { right=right.max(mx1.saturating_sub(right_edge.max(mx0))); }
        top=top.max(strut.top.saturating_sub(my0).min(monitor.height));
        let bottom_edge=root_height.saturating_sub(strut.bottom);
        if bottom_edge < my1 { bottom=bottom.max(my1.saturating_sub(bottom_edge.max(my0))); }
    }
    left=left.min(monitor.width.saturating_sub(1));
    right=right.min(monitor.width.saturating_sub(left).saturating_sub(1));
    top=top.min(monitor.height.saturating_sub(1));
    bottom=bottom.min(monitor.height.saturating_sub(top).saturating_sub(1));
    mrx_core::layout::Rect {
        x: monitor.x.saturating_add(left as i32),
        y: monitor.y.saturating_add(top as i32),
        width: monitor.width.saturating_sub(left).saturating_sub(right).max(1),
        height: monitor.height.saturating_sub(top).saturating_sub(bottom).max(1),
    }
}

fn publish_workarea(
    conn: &RustConnection,
    root: Window,
    workarea_atom: u32,
    state: &State,
) -> Result<()> {
    let root_monitor=MonitorInfo {
        name: "root-workarea".into(), x: 0, y: 0,
        width: state.root_width, height: state.root_height,
        focused: true, active_workspace: 1,
    };
    let area=monitor_workarea(&root_monitor,&state.docks,state.root_width,state.root_height);
    let values:Vec<u32>=(0..state.settings.workspace.count)
        .flat_map(|_|[area.x.max(0) as u32,area.y.max(0) as u32,area.width,area.height])
        .collect();
    conn.change_property32(xproto::PropMode::REPLACE,root,workarea_atom,AtomEnum::CARDINAL,&values)?;
    Ok(())
}


fn publish_client_lists(
    conn: &RustConnection,
    root: Window,
    client_list: u32,
    client_list_stacking: u32,
    state: &State,
) -> Result<()> {
    let mapped_order: Vec<u32> = state.client_order.iter()
        .copied()
        .filter(|id| state.clients.contains_key(id))
        .collect();
    conn.change_property32(
        xproto::PropMode::REPLACE, root, client_list, AtomEnum::WINDOW, &mapped_order,
    )?;

    let tree = conn.query_tree(root)?.reply()?;
    let stacking: Vec<u32> = tree.children.into_iter()
        .filter(|id| state.clients.contains_key(id))
        .collect();
    conn.change_property32(
        xproto::PropMode::REPLACE, root, client_list_stacking, AtomEnum::WINDOW, &stacking,
    )?;
    Ok(())
}


/// Return a valid non-self ICCCM transient parent, if one is advertised.
fn read_transient_for(conn: &RustConnection, window: Window) -> Option<Window> {
    conn.get_property(false, window, AtomEnum::WM_TRANSIENT_FOR, AtomEnum::WINDOW, 0, 1).ok()?
        .reply().ok()?.value32()?.next().filter(|parent|*parent!=window)
}

fn is_dock_window(conn: &RustConnection, window: Window, window_type_atom: u32, dock_atom: u32) -> bool {
    conn.get_property(false,window,window_type_atom,AtomEnum::ATOM,0,16).ok()
        .and_then(|cookie|cookie.reply().ok())
        .and_then(|reply|reply.value32().map(|mut atoms|atoms.any(|atom|atom==dock_atom)))
        .unwrap_or(false)
}
fn main()->Result<()> { tracing_subscriber::fmt().with_env_filter("info").init(); let (conn,screen_num)=x11rb::connect(None).context("cannot connect to X11 display")?; let screen=&conn.setup().roots[screen_num]; let root=screen.root; conn.change_window_attributes(root,&ChangeWindowAttributesAux::new().event_mask(EventMask::SUBSTRUCTURE_REDIRECT|EventMask::SUBSTRUCTURE_NOTIFY|EventMask::PROPERTY_CHANGE|EventMask::BUTTON_PRESS)).context("another window manager may already own SubstructureRedirect")?.check()?;
 let supported=intern(&conn,"_NET_SUPPORTED")?; let wmcheck=intern(&conn,"_NET_SUPPORTING_WM_CHECK")?; let name=intern(&conn,"_NET_WM_NAME")?; let utf8=intern(&conn,"UTF8_STRING")?; let current=intern(&conn,"_NET_CURRENT_DESKTOP")?; let count=intern(&conn,"_NET_NUMBER_OF_DESKTOPS")?; let clientlist=intern(&conn,"_NET_CLIENT_LIST")?; let clientlist_stacking=intern(&conn,"_NET_CLIENT_LIST_STACKING")?; let desktop=intern(&conn,"_NET_WM_DESKTOP")?; let wm_protocols=intern(&conn,"WM_PROTOCOLS")?; let wm_hints=intern(&conn,"WM_HINTS")?; let wm_normal_hints=intern(&conn,"WM_NORMAL_HINTS")?; let net_wm_window_type=intern(&conn,"_NET_WM_WINDOW_TYPE")?; let net_wm_window_type_dock=intern(&conn,"_NET_WM_WINDOW_TYPE_DOCK")?; let net_wm_strut=intern(&conn,"_NET_WM_STRUT")?; let net_wm_strut_partial=intern(&conn,"_NET_WM_STRUT_PARTIAL")?; let wm_delete=intern(&conn,"WM_DELETE_WINDOW")?; let wm_take_focus=intern(&conn,"WM_TAKE_FOCUS")?; let wm_state=intern(&conn,"WM_STATE")?; let net_close=intern(&conn,"_NET_CLOSE_WINDOW")?; let net_active=intern(&conn,"_NET_ACTIVE_WINDOW")?; let net_wm_state=intern(&conn,"_NET_WM_STATE")?; let net_wm_state_fullscreen=intern(&conn,"_NET_WM_STATE_FULLSCREEN")?; let net_wm_state_hidden=intern(&conn,"_NET_WM_STATE_HIDDEN")?; let wm_change_state=intern(&conn,"WM_CHANGE_STATE")?; let net_wm_state_maximized_horz=intern(&conn,"_NET_WM_STATE_MAXIMIZED_HORZ")?; let net_wm_state_maximized_vert=intern(&conn,"_NET_WM_STATE_MAXIMIZED_VERT")?; let net_wm_state_above=intern(&conn,"_NET_WM_STATE_ABOVE")?; let net_wm_state_below=intern(&conn,"_NET_WM_STATE_BELOW")?; let net_wm_state_modal=intern(&conn,"_NET_WM_STATE_MODAL")?; let wm_transient_for=intern(&conn,"WM_TRANSIENT_FOR")?; let net_desktop_names=intern(&conn,"_NET_DESKTOP_NAMES")?; let net_workarea=intern(&conn,"_NET_WORKAREA")?; let net_viewport=intern(&conn,"_NET_DESKTOP_VIEWPORT")?; let net_geometry=intern(&conn,"_NET_DESKTOP_GEOMETRY")?; let net_active_window=intern(&conn,"_NET_ACTIVE_WINDOW")?; let workspace_atoms=WorkspaceAtoms{current,wm_state,net_wm_state,net_wm_state_hidden,net_active_window};
 let atoms=[net_active,net_close,desktop,current,count,clientlist,clientlist_stacking,net_wm_state,net_wm_state_fullscreen,net_wm_state_hidden,net_wm_state_maximized_horz,net_wm_state_maximized_vert,net_wm_state_above,net_wm_state_below,net_wm_state_modal,net_desktop_names,net_workarea,net_viewport,net_geometry,net_wm_window_type,net_wm_window_type_dock,net_wm_strut,net_wm_strut_partial]; conn.change_property32(xproto::PropMode::REPLACE,root,supported,AtomEnum::ATOM,&atoms)?; conn.change_property8(xproto::PropMode::REPLACE,root,name,utf8,b"MRX Rust Desktop")?; let check=conn.generate_id()?; conn.create_window(x11rb::COPY_DEPTH_FROM_PARENT,check,root,0,0,1,1,0,xproto::WindowClass::INPUT_OUTPUT,0,&xproto::CreateWindowAux::new())?; conn.change_property32(xproto::PropMode::REPLACE,root,wmcheck,AtomEnum::WINDOW,&[check])?; conn.change_property32(xproto::PropMode::REPLACE,check,wmcheck,AtomEnum::WINDOW,&[check])?; conn.change_property8(xproto::PropMode::REPLACE,check,name,utf8,b"MRX Rust Desktop")?;
 let mut monitors = discover_monitors(&conn, root, screen.width_in_pixels, screen.height_in_pixels);
  let primary = monitors.iter().find(|m| m.focused).or_else(|| monitors.first()).map(|m| m.name.clone()).unwrap_or_default();
  for monitor in &mut monitors { monitor.focused = monitor.name == primary; monitor.active_workspace = 1; }
  let active_by_monitor = monitors.iter().map(|m| (m.name.clone(), 1)).collect();
  let settings = Settings::load().unwrap_or_default();
  let state=Arc::new(Mutex::new(State{active:1,settings,monitors,active_by_monitor,focused_monitor:primary,root_width:screen.width_in_pixels as u32,root_height:screen.height_in_pixels as u32,started_at:Some(Instant::now()),..Default::default()}));
  let workspace_count=state.lock().unwrap().settings.workspace.count as u32;
 conn.change_property32(xproto::PropMode::REPLACE,root,count,AtomEnum::CARDINAL,&[workspace_count])?;
 publish_workarea(&conn,root,net_workarea,&state.lock().unwrap())?;
 if conn.extension_information(randr::X11_EXTENSION_NAME)?.is_some() {
     let mask = NotifyMask::SCREEN_CHANGE | NotifyMask::CRTC_CHANGE | NotifyMask::OUTPUT_CHANGE | NotifyMask::RESOURCE_CHANGE;
     match conn.randr_select_input(root, mask) {
         Ok(cookie) => if let Err(error) = cookie.check() { warn!(%error, "RandR change notifications unavailable"); },
         Err(error) => warn!(%error, "RandR change notifications unavailable"),
     }
 }
 let settings_snapshot=state.lock().unwrap().settings.clone();
 let mut desktop_names=Vec::new();
 for index in 0..workspace_count as usize { let label=settings_snapshot.workspace.names.get(index).cloned().unwrap_or_else(|| (index+1).to_string()); desktop_names.extend_from_slice(label.as_bytes()); desktop_names.push(0); }
 conn.change_property8(xproto::PropMode::REPLACE,root,net_desktop_names,utf8,&desktop_names)?;
 publish_workarea(&conn,root,net_workarea,&state.lock().unwrap())?;
 conn.change_property32(xproto::PropMode::REPLACE,root,net_viewport,AtomEnum::CARDINAL,&(0..workspace_count).flat_map(|_| [0u32,0]).collect::<Vec<_>>())?;
 conn.change_property32(xproto::PropMode::REPLACE,root,net_geometry,AtomEnum::CARDINAL,&[screen.width_in_pixels as u32,screen.height_in_pixels as u32])?;
 // Adopt already-viewable top-level clients when MRX starts. The X server's
 // child list is bottom-to-top stacking order; it is the best available order
 // when the WM starts after clients have already mapped.
 {
   let tree=conn.query_tree(root)?.reply()?;
   let mut s=state.lock().unwrap();
   for window in tree.children {
     if window==check { continue; }
     let attributes=match conn.get_window_attributes(window)?.reply() { Ok(value)=>value, Err(_)=>continue };
     if attributes.override_redirect { continue; }
     let geometry=match conn.get_geometry(window)?.reply() { Ok(value)=>value, Err(_)=>continue };
     let title=conn.get_property(false,window,AtomEnum::WM_NAME,AtomEnum::STRING,0,1024)?.reply()?.value;
     let size_hints = read_size_hints(&conn, window);
     let dock = is_dock_window(&conn,window,net_wm_window_type,net_wm_window_type_dock);
     let transient_for=read_transient_for(&conn,window).unwrap_or(0);
     let floating = dock || transient_for!=0 || has_fixed_size(size_hints.as_ref());
     let title=String::from_utf8_lossy(&title).into_owned();
     let existing_states=read_atom_list(&conn,window,net_wm_state);
     let existing_wm_state=read_cardinal(&conn,window,wm_state,wm_state).unwrap_or(1);
     let desktop_index=read_cardinal(&conn,window,desktop,AtomEnum::CARDINAL.into())
         .filter(|value|*value!=u32::MAX && *value<workspace_count)
         .map(|value|value+1);
     // HIDDEN also marks clients on inactive desktops; only infer minimization
     // from HIDDEN alone when the client's desktop is currently active.
     let hidden=existing_states.contains(&net_wm_state_hidden);
     let minimized=existing_wm_state==3 || (hidden && desktop_index.is_none_or(|workspace|workspace==1));
     let desktop_hidden=existing_wm_state==1 && hidden && desktop_index.is_some();
     if attributes.map_state==xproto::MapState::UNMAPPED && !minimized && !desktop_hidden { continue; }
     let _=conn.change_window_attributes(window,&ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE));
     if dock {
         if let Some(strut)=read_dock_strut(&conn,window,net_wm_strut_partial,net_wm_strut) { s.docks.insert(window,strut); }
     }
     s.client_order.push(window);
     if let Some(hints) = size_hints { s.size_hints.insert(window, hints); }
     let assigned_monitor=monitor_for_geometry(&s.monitors,geometry.x as i32,geometry.y as i32,geometry.width as u32,geometry.height as u32);
     let workspace=desktop_index.unwrap_or_else(||active_workspace_for(&s,&assigned_monitor));
     let fullscreen=existing_states.contains(&net_wm_state_fullscreen);
     let maximized_horz=existing_states.contains(&net_wm_state_maximized_horz);
     let maximized_vert=existing_states.contains(&net_wm_state_maximized_vert);
     let above=existing_states.contains(&net_wm_state_above);
     let below=existing_states.contains(&net_wm_state_below) && !above;
     let modal=existing_states.contains(&net_wm_state_modal);
     s.clients.insert(window,WindowInfo{
         id:window,title,class:String::new(),workspace,monitor:assigned_monitor,
         x:geometry.x as i32,y:geometry.y as i32,width:geometry.width as u32,height:geometry.height as u32,
         focused:false,floating,dock,minimized,fullscreen,maximized_horz,maximized_vert,above,below,
         transient_for,modal,urgent:read_urgent_hint(&conn,window)
     });
     if fullscreen || maximized_horz || maximized_vert {
         let original=mrx_core::layout::Rect{x:geometry.x as i32,y:geometry.y as i32,width:geometry.width as u32,height:geometry.height as u32};
         s.restore_geometry.insert(window,(original,floating));
         let area=s.monitors.iter().find(|monitor|monitor.name==s.clients[&window].monitor)
             .map(|monitor|monitor_workarea(monitor,&s.docks,s.root_width,s.root_height))
             .unwrap_or(mrx_core::layout::Rect{x:0,y:0,width:s.root_width.max(1),height:s.root_height.max(1)});
         let target=if fullscreen { area } else { maximized_rect(original,area,maximized_horz,maximized_vert) };
         if let Some(client)=s.clients.get_mut(&window) { client.floating=true; client.x=target.x; client.y=target.y; client.width=target.width; client.height=target.height; }
         let border=if fullscreen {0}else{s.settings.appearance.border_width};
         let _=conn.configure_window(window,&ConfigureWindowAux::new().x(target.x).y(target.y).width(target.width).height(target.height).border_width(border));
     }
     let _=set_wm_state(&conn,window,wm_state,if minimized {3}else{1});
     let desktop_value=if dock {u32::MAX} else {workspace.saturating_sub(1)};
     let _=conn.change_property32(xproto::PropMode::REPLACE,window,desktop,AtomEnum::CARDINAL,&[desktop_value]);
     let visible=!minimized && (dock || workspace==active_workspace_for(&s,&s.clients[&window].monitor));
     if visible { let _=conn.map_window(window); } else { let _=conn.unmap_window(window); }
     let _=set_atom_state(&conn,window,net_wm_state,net_wm_state_hidden,minimized || (!dock && !visible));
     let _=set_atom_state(&conn,window,net_wm_state,net_wm_state_above,above);
     let _=set_atom_state(&conn,window,net_wm_state,net_wm_state_below,below);
   }
   // Restore the requested EWMH layer ordering after adoption.
   restack_managed_clients(&conn,&s);
   if let Some(topmost)=preferred_focus_client(&s) {
     if let Some(monitor)=s.clients.get(&topmost).map(|client|client.monitor.clone()) { set_focused_monitor(&mut s,&monitor); }
     if let Some(client)=s.clients.get_mut(&topmost) { client.focused=true; }
     let _=focus_client(&conn,topmost,wm_protocols,wm_take_focus);
     let _=conn.change_property32(xproto::PropMode::REPLACE,root,net_active_window,AtomEnum::WINDOW,&[topmost]);
   }
   publish_workarea(&conn,root,net_workarea,&s)?;
   publish_client_lists(&conn,root,clientlist,clientlist_stacking,&s)?;
   arrange(&conn,root,&s);
 }
 let (tx,rx)=mpsc::channel(); let ipc_state=state.clone(); thread::spawn(move||if let Err(e)=serve_ipc(ipc_state,tx){warn!(error=%e,"IPC server stopped")});
 for key in 10..=18u8 { let _=conn.grab_key(false,root,ModMask::M4,key,GrabMode::ASYNC,GrabMode::ASYNC); }
 let _=conn.grab_key(false,root,ModMask::M4,24,GrabMode::ASYNC,GrabMode::ASYNC); // Super+Q closes focused window
 let _=conn.grab_key(false,root,ModMask::M4,113,GrabMode::ASYNC,GrabMode::ASYNC); // Super+Left
 let _=conn.grab_key(false,root,ModMask::M4,114,GrabMode::ASYNC,GrabMode::ASYNC); // Super+Right
 conn.change_property32(xproto::PropMode::REPLACE,root,current,AtomEnum::CARDINAL,&[0])?;
 // Client lists were already published after startup adoption; do not erase them here.
 conn.flush()?; info!("MRX X11 window manager started");
 let mut last_monitor_refresh = std::time::Instant::now();
 loop {
   let mut should_shutdown=false;
   while let Ok(received)=rx.try_recv() {
     let (operation_id,action)=match received {
       Action::Operation(id,action)=>(Some(id),*action),
       action=>(None,action),
     };
     let mut s=state.lock().unwrap();
     let mut failure:Option<OperationError>=None;
     match action {
       Action::SwitchMonitor(name,id)=>{
         if id==0 || id>s.settings.workspace.count as u32 {
           failure=Some(OperationError{code:"invalid_workspace".into(),message:format!("workspace {id} is outside the configured range"),retryable:false});
         } else if !s.monitors.iter().any(|monitor|monitor.name==name) {
           failure=Some(OperationError{code:"monitor_not_found".into(),message:format!("monitor '{name}' was not found"),retryable:false});
         } else { switch_monitor_workspace(&conn,root,&mut s,&name,id,workspace_atoms); }
       },
       Action::Switch(id)=>{
         if id==0 || id>s.settings.workspace.count as u32 {
           failure=Some(OperationError{code:"invalid_workspace".into(),message:format!("workspace {id} is outside the configured range"),retryable:false});
         } else { switch_focused_workspace(&conn,root,&mut s,id,None,workspace_atoms); }
       },
       Action::Relative(direction)=>{
         if direction!=1 && direction!=-1 {
           failure=Some(OperationError{code:"invalid_direction".into(),message:"relative workspace direction must be -1 or 1".into(),retryable:false});
         } else { switch_focused_workspace(&conn,root,&mut s,0,Some(direction),workspace_atoms); }
       },
       Action::Focus(requested)=>{
         let id=modal_focus_target(&s,requested);
         let target_monitor=s.clients.get(&id).map(|window|window.monitor.clone());
         match target_monitor {
           None=>failure=Some(OperationError{code:"window_not_found".into(),message:format!("window {requested} is not managed"),retryable:false}),
           Some(monitor) if s.clients.get(&id).is_some_and(|window|window.dock || window.minimized || window.workspace!=active_workspace_for(&s,&monitor))=>
             failure=Some(OperationError{code:"window_not_visible".into(),message:format!("window {id} is not focusable on the active workspace"),retryable:false}),
           Some(monitor)=>{
             set_focused_monitor(&mut s,&monitor);
             for window in s.clients.values_mut(){window.focused=window.id==id;}
             if let Err(error)=focus_client(&conn,id,wm_protocols,wm_take_focus) {
               failure=Some(OperationError{code:"focus_failed".into(),message:error.to_string(),retryable:true});
             } else {
               let _=conn.change_property32(xproto::PropMode::REPLACE,root,net_active_window,AtomEnum::WINDOW,&[id]);
             }
           }
         }
       },
       Action::Close(id)=>{
         if !s.clients.contains_key(&id) {
           failure=Some(OperationError{code:"window_not_found".into(),message:format!("window {id} is not managed"),retryable:false});
         } else if let Err(error)=request_close(&conn,id,wm_protocols,wm_delete) {
           failure=Some(OperationError{code:"close_failed".into(),message:error.to_string(),retryable:true});
         }
       },
       Action::Move(id,workspace)=>{
         if workspace==0 || workspace>s.settings.workspace.count as u32 {
           failure=Some(OperationError{code:"invalid_workspace".into(),message:format!("workspace {workspace} is outside the configured range"),retryable:false});
         } else if !s.clients.contains_key(&id) {
           failure=Some(OperationError{code:"window_not_found".into(),message:format!("window {id} is not managed"),retryable:false});
         } else if s.clients.get(&id).is_some_and(|window|window.dock) {
           failure=Some(OperationError{code:"window_is_sticky".into(),message:"dock clients are sticky and cannot be moved between workspaces".into(),retryable:false});
         } else {
           let minimized=s.clients.get(&id).is_some_and(|window|window.minimized);
           let visible=s.clients.get(&id).is_some_and(|window|!window.minimized && workspace==active_workspace_for(&s,&window.monitor));
           if let Some(window)=s.clients.get_mut(&id) {
             window.workspace=workspace;
             let _=conn.change_property32(xproto::PropMode::REPLACE,id,desktop,AtomEnum::CARDINAL,&[workspace-1]);
             if minimized {
               let _=set_wm_state(&conn,id,wm_state,3);
               let _=set_atom_state(&conn,id,net_wm_state,net_wm_state_hidden,true);
             } else if visible {
               let _=conn.map_window(id);
               let _=set_wm_state(&conn,id,wm_state,1);
               let _=set_atom_state(&conn,id,net_wm_state,net_wm_state_hidden,false);
             } else {
               let _=conn.unmap_window(id);
               let _=set_wm_state(&conn,id,wm_state,1);
               let _=set_atom_state(&conn,id,net_wm_state,net_wm_state_hidden,true);
             }
           }
           arrange(&conn,root,&s);
         }
       },
       Action::Set(key,value)=>{
         let previous=s.settings.clone();
         if let Err(error)=apply_setting(&mut s,&key,value) {
           failure=Some(OperationError{code:"invalid_setting".into(),message:error.to_string(),retryable:false});
         } else {
           push_settings_history(&mut s,previous);
           for window in s.clients.values(){let _=conn.configure_window(window.id,&ConfigureWindowAux::new().border_width(s.settings.appearance.border_width));}
           arrange(&conn,root,&s);
         }
       },
       Action::UndoSettings=>{
         if let Some(previous)=s.settings_history.last().cloned() {
           match previous.save_atomic() {
             Ok(())=>{s.settings_history.pop();s.settings=previous;arrange(&conn,root,&s);}
             Err(error)=>failure=Some(OperationError{code:"settings_persist_failed".into(),message:error.to_string(),retryable:true}),
           }
         } else {
           failure=Some(OperationError{code:"no_undo_available".into(),message:"there are no previous settings to restore".into(),retryable:false});
         }
       },
       Action::ResetSettings=>{
         let defaults=Settings::default();
         let previous=s.settings.clone();
         match defaults.save_atomic() {
           Ok(())=>{if defaults!=previous{push_settings_history(&mut s,previous);}s.settings=defaults;arrange(&conn,root,&s);}
           Err(error)=>failure=Some(OperationError{code:"settings_persist_failed".into(),message:error.to_string(),retryable:true}),
         }
       },
       Action::ExportProfile(name)=>{
         if let Err(error)=s.settings.save_profile_atomic(&name) {
           failure=Some(OperationError{code:"profile_export_failed".into(),message:error.to_string(),retryable:false});
         }
       },
       Action::ImportProfile(name)=>{
         match Settings::load_profile(&name) {
           Ok(imported)=>{
             match imported.save_atomic() {
               Ok(())=>{let previous=s.settings.clone();push_settings_history(&mut s,previous);s.settings=imported;arrange(&conn,root,&s);}
               Err(error)=>failure=Some(OperationError{code:"settings_persist_failed".into(),message:error.to_string(),retryable:true}),
             }
           },
           Err(error)=>failure=Some(OperationError{code:"profile_import_failed".into(),message:error.to_string(),retryable:false}),
         }
       },
       Action::Reload=>{
         match Settings::load() {
           Ok(settings)=>{s.settings=settings;arrange(&conn,root,&s);}
           Err(error)=>failure=Some(OperationError{code:"settings_reload_failed".into(),message:error.to_string(),retryable:true}),
         }
       },
       Action::Shutdown=>{should_shutdown=true;},
       Action::Operation(_,_)=>failure=Some(OperationError{code:"invalid_operation".into(),message:"nested operation envelopes are not supported".into(),retryable:false}),
     }
     if let Some(id)=operation_id { finish_operation(&mut s,id,failure); }
   }
   if should_shutdown { info!("orderly shutdown requested over IPC"); break; }
   match conn.poll_for_event()? {
    Some(Event::RandrNotify(_)) => {
        let mut s=state.lock().unwrap();
        let width=s.root_width.min(u16::MAX as u32) as u16;
        let height=s.root_height.min(u16::MAX as u32) as u16;
        refresh_monitor_state(&conn,root,&mut s,width,height);
        if let Err(error)=publish_workarea(&conn,root,net_workarea,&s){warn!(%error,"failed to refresh workarea after RandR resource change");}
    },
    Some(Event::RandrScreenChangeNotify(event)) => {
        let mut s=state.lock().unwrap();
        refresh_monitor_state(&conn,root,&mut s,event.width,event.height);
        if let Err(error)=publish_workarea(&conn,root,net_workarea,&s){warn!(%error,"failed to refresh workarea after screen resize");}
    },
    Some(Event::PropertyNotify(e))=>{
        if e.atom == wm_hints || e.atom == wm_normal_hints || e.atom == net_wm_strut || e.atom == net_wm_strut_partial || e.atom == net_wm_window_type || e.atom == wm_transient_for || e.atom == net_wm_state {
            let urgent = if e.atom == wm_hints { Some(read_urgent_hint(&conn, e.window)) } else { None };
            let size_hints = if e.atom == wm_normal_hints { Some(read_size_hints(&conn, e.window)) } else { None };
            let transient_for = if e.atom == wm_transient_for { Some(read_transient_for(&conn,e.window).unwrap_or(0)) } else { None };
            let modal = if e.atom == net_wm_state { Some(read_atom_list(&conn,e.window,net_wm_state).contains(&net_wm_state_modal)) } else { None };
            let mut s = state.lock().unwrap();
            if let Some(urgent) = urgent {
                if let Some(window) = s.clients.get_mut(&e.window) { window.urgent = urgent; }
            }
            if let Some(size_hints_value) = size_hints.as_ref() {
                if let Some(hints) = size_hints_value.as_ref() { s.size_hints.insert(e.window, *hints); }
                else { s.size_hints.remove(&e.window); }
            }
            if let Some(parent) = transient_for {
                if let Some(window) = s.clients.get_mut(&e.window) { window.transient_for=parent; }
            }
            if let Some(is_modal) = modal {
                if let Some(window) = s.clients.get_mut(&e.window) { window.modal=is_modal; }
            }
            if transient_for.is_some() || size_hints.is_some() {
                let fixed = s.size_hints.get(&e.window).is_some_and(|h|has_fixed_size(Some(h)));
                let parent = s.clients.get(&e.window).is_some_and(|w|w.transient_for!=0);
                if let Some(window) = s.clients.get_mut(&e.window) { window.floating=window.dock || parent || fixed; }
            }
            if modal==Some(true) {
                for window in s.clients.values_mut(){window.focused=false;}
                if let Some(target)=preferred_focus_client(&s) {
                    if let Err(error)=focus_client(&conn,target,wm_protocols,wm_take_focus){warn!(window=target,%error,"modal dialog focus negotiation failed");}
                    let _=conn.change_property32(xproto::PropMode::REPLACE,root,net_active_window,AtomEnum::WINDOW,&[target]);
                }
            }
            if e.atom == net_wm_strut || e.atom == net_wm_strut_partial || e.atom == net_wm_window_type {
                let dock = is_dock_window(&conn,e.window,net_wm_window_type,net_wm_window_type_dock);
                if dock {
                    if let Some(strut)=read_dock_strut(&conn,e.window,net_wm_strut_partial,net_wm_strut) { s.docks.insert(e.window,strut); }
                    else { s.docks.remove(&e.window); }
                } else {
                    s.docks.remove(&e.window);
                }
                let fixed = s.size_hints.get(&e.window).is_some_and(|h|has_fixed_size(Some(h)));
                let transient = s.clients.get(&e.window).is_some_and(|window|window.transient_for!=0);
                let was_focused=s.clients.get(&e.window).is_some_and(|window|window.focused);
                if let Some(window)=s.clients.get_mut(&e.window) {
                    window.dock=dock;
                    window.floating=dock || transient || fixed;
                    if dock { window.focused=false; }
                }
                if let Some(window)=s.clients.get(&e.window) {
                    let desktop_value=if window.dock {u32::MAX} else {window.workspace.saturating_sub(1)};
                    let _=conn.change_property32(xproto::PropMode::REPLACE,e.window,desktop,AtomEnum::CARDINAL,&[desktop_value]);
                }
                if dock && was_focused {
                    focus_most_recent(&conn,root,&mut s,wm_protocols,wm_take_focus,net_active_window);
                }
                if let Err(error)=publish_workarea(&conn,root,net_workarea,&s) { warn!(%error,"failed to update EWMH workarea after dock change"); }
            }
            arrange(&conn, root, &s);
        }
     }, Some(Event::MapRequest(e))=>{
        info!(window=e.window,"received X11 MapRequest");
        let mut s=state.lock().unwrap();
        if let Err(error)=conn.map_window(e.window) { warn!(window=e.window,%error,"failed to map requested client"); continue; }
        let _=conn.configure_window(e.window,&ConfigureWindowAux::new().border_width(s.settings.appearance.border_width));
        let g=match conn.get_geometry(e.window) { Ok(cookie)=>match cookie.reply(){Ok(geometry)=>geometry,Err(error)=>{warn!(window=e.window,%error,"failed to query requested client geometry");continue;}},Err(error)=>{warn!(window=e.window,%error,"failed to request client geometry");continue;} };
        let size_hints = read_size_hints(&conn, e.window);
         let dock = is_dock_window(&conn,e.window,net_wm_window_type,net_wm_window_type_dock);
         let transient_for=read_transient_for(&conn,e.window).unwrap_or(0);
         let initial_states=read_atom_list(&conn,e.window,net_wm_state);
         let modal=initial_states.contains(&net_wm_state_modal);
         let floating = dock || transient_for!=0 || has_fixed_size(size_hints.as_ref());
        let title=conn.get_property(false,e.window,AtomEnum::WM_NAME,AtomEnum::STRING,0,1024).ok()
            .and_then(|cookie|cookie.reply().ok()).map(|reply|String::from_utf8_lossy(&reply.value).to_string()).unwrap_or_default();
        if !dock { for client in s.clients.values_mut(){client.focused=false;} }
        if !s.clients.contains_key(&e.window) { s.client_order.push(e.window); }
         if let Some(hints) = size_hints { s.size_hints.insert(e.window, hints); }
        let assigned_monitor=conn.query_pointer(root).ok().and_then(|cookie|cookie.reply().ok()).map(|pointer|monitor_for_geometry(&s.monitors,pointer.root_x as i32,pointer.root_y as i32,1,1)).unwrap_or_else(||monitor_for_geometry(&s.monitors,g.x as i32,g.y as i32,g.width as u32,g.height as u32));
        if !dock { set_focused_monitor(&mut s,&assigned_monitor); }
        let ws=active_workspace_for(&s,&assigned_monitor);
        s.clients.insert(e.window,WindowInfo{id:e.window,title,class:String::new(),workspace:ws,monitor:assigned_monitor,x:g.x as i32,y:g.y as i32,width:g.width as u32,height:g.height as u32,focused:!dock,floating,urgent:read_urgent_hint(&conn,e.window),dock,transient_for,modal,..Default::default()});
        let _=conn.change_window_attributes(e.window,&ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE));
        if is_dock_window(&conn,e.window,net_wm_window_type,net_wm_window_type_dock) {
            if let Some(strut)=read_dock_strut(&conn,e.window,net_wm_strut_partial,net_wm_strut) { s.docks.insert(e.window,strut); }
        }
        if !dock {
            let focus_target=modal_focus_target(&s,e.window);
            for client in s.clients.values_mut(){client.focused=client.id==focus_target;}
            if let Err(error)=focus_client(&conn,focus_target,wm_protocols,wm_take_focus){warn!(window=focus_target,%error,"ICCCM focus negotiation failed");}
            let _=conn.change_property32(xproto::PropMode::REPLACE,root,net_active_window,AtomEnum::WINDOW,&[focus_target]);
        } else {
            let _=conn.change_property32(xproto::PropMode::REPLACE,e.window,desktop,AtomEnum::CARDINAL,&[u32::MAX]);
            if let Err(error)=publish_workarea(&conn,root,net_workarea,&s){warn!(%error,"failed to publish workarea for mapped dock");}
        }
        let _=set_wm_state(&conn,e.window,wm_state,1);
        if !dock { let _=conn.change_property32(xproto::PropMode::REPLACE,e.window,desktop,AtomEnum::CARDINAL,&[ws-1]); }
        restack_managed_clients(&conn,&s);
        if let Err(error)=publish_client_lists(&conn,root,clientlist,clientlist_stacking,&s){warn!(%error,"failed to publish EWMH client lists");}
        arrange(&conn,root,&s);
    }, Some(Event::UnmapNotify(e))=>{
        let mut s=state.lock().unwrap();
        let dock_removed=s.docks.remove(&e.window).is_some();
        let removed=s.clients.get(&e.window).is_some_and(|w|!w.minimized && (w.dock || w.workspace==active_workspace_for(&s,&w.monitor)));
        if removed {
            s.clients.remove(&e.window);s.size_hints.remove(&e.window);s.restore_geometry.remove(&e.window);s.client_order.retain(|id|*id!=e.window);
            let _=set_wm_state(&conn,e.window,wm_state,0);
            if let Err(error)=publish_client_lists(&conn,root,clientlist,clientlist_stacking,&s){warn!(%error,"failed to publish EWMH client lists");}
            focus_most_recent(&conn,root,&mut s,wm_protocols,wm_take_focus,net_active_window);
        }
        restack_managed_clients(&conn,&s);
        if dock_removed { let _=publish_workarea(&conn,root,net_workarea,&s); }
        arrange(&conn,root,&s);
     }, Some(Event::DestroyNotify(e))=>{let mut s=state.lock().unwrap();s.clients.remove(&e.window);s.size_hints.remove(&e.window);s.restore_geometry.remove(&e.window);s.docks.remove(&e.window);s.client_order.retain(|id|*id!=e.window);let _=publish_workarea(&conn,root,net_workarea,&s);if let Err(error)=publish_client_lists(&conn,root,clientlist,clientlist_stacking,&s){warn!(%error,"failed to publish EWMH client lists");}focus_most_recent(&conn,root,&mut s,wm_protocols,wm_take_focus,net_active_window);restack_managed_clients(&conn,&s);arrange(&conn,root,&s);}, Some(Event::ConfigureRequest(e))=>{
       let managed = state.lock().unwrap().clients.get(&e.window).cloned();
       if let Some(window) = managed {
         if window.floating && !window.fullscreen {
           let aux=ConfigureWindowAux::new().x(e.x as i32).y(e.y as i32).width(e.width as u32).height(e.height as u32).border_width(e.border_width as u32).sibling(e.sibling).stack_mode(e.stack_mode);
           let _=conn.configure_window(e.window,&aux);
           let s=state.lock().unwrap();
           restack_managed_clients(&conn,&s);
         } else { let s=state.lock().unwrap(); arrange(&conn,root,&s); }
       } else {
         let aux=ConfigureWindowAux::new().x(e.x as i32).y(e.y as i32).width(e.width as u32).height(e.height as u32).border_width(e.border_width as u32).sibling(e.sibling).stack_mode(e.stack_mode);
         let _=conn.configure_window(e.window,&aux);
       }
     }, Some(Event::KeyPress(e))=>{
        let key=e.detail;
        if (10..=18).contains(&key) {
            let mut s=state.lock().unwrap();
            for w in s.clients.values_mut(){w.focused=false;}
            switch_focused_workspace(&conn,root,&mut s,(key-9) as u32,None,workspace_atoms);
        } else if key==113 || key==114 {
            let mut s=state.lock().unwrap();
            let delta=if key==114 {1} else {-1};
            for w in s.clients.values_mut(){w.focused=false;}
            switch_focused_workspace(&conn,root,&mut s,0,Some(delta),workspace_atoms);
        } else if key==24 {
            let s=state.lock().unwrap();
            if let Some(w)=s.clients.values().find(|w|w.focused) {
                if let Err(e)=request_close(&conn,w.id,wm_protocols,wm_delete){warn!(error=%e,"polite close failed")}
            }
        }
    }, Some(Event::ClientMessage(e))=>{
       let data=e.data.as_data32();
       if e.type_==net_close { let target=e.window; if target!=0 && target!=root { if let Err(err)=request_close(&conn,target,wm_protocols,wm_delete){warn!(window=target,error=%err,"EWMH close request failed");} } }
       else if e.type_==net_active {
         let requested=e.window;
         let mut s=state.lock().unwrap();
         let target=modal_focus_target(&s,requested);
         let target_monitor=s.clients.get(&target).map(|w|w.monitor.clone());
         if let Some(monitor)=target_monitor {
             if s.clients.get(&target).is_some_and(|w|!w.dock && w.workspace==active_workspace_for(&s,&monitor)) {
                 if let Some(window)=s.clients.get_mut(&target) { window.minimized=false; }
                 let _=set_wm_state(&conn,target,wm_state,1);
                 let _=set_atom_state(&conn,target,net_wm_state,net_wm_state_hidden,false);
                 set_focused_monitor(&mut s,&monitor);
                 for w in s.clients.values_mut(){w.focused=w.id==target;}
                 let _=conn.map_window(target);
                 if let Err(error)=focus_client(&conn,target,wm_protocols,wm_take_focus){warn!(window=target,%error,"ICCCM focus negotiation failed");}
                 let _=conn.change_property32(xproto::PropMode::REPLACE,root,net_active_window,AtomEnum::WINDOW,&[target]);
             }
         }
       }
       else if e.type_==wm_change_state {
         let target=e.window;
         let minimize=data[0]==3;
         let mut s=state.lock().unwrap();
         let visible=s.clients.get(&target).is_some_and(|w|!w.dock && w.workspace==active_workspace_for(&s,&w.monitor));
         if let Some(window)=s.clients.get_mut(&target) {
           window.minimized=minimize;
           if minimize { window.focused=false; }
         }
         if minimize {
           let _=conn.unmap_window(target);
           let _=set_wm_state(&conn,target,wm_state,3);
           let _=set_atom_state(&conn,target,net_wm_state,net_wm_state_hidden,true);
           focus_most_recent(&conn,root,&mut s,wm_protocols,wm_take_focus,net_active_window);
         } else if visible {
           let _=conn.map_window(target);
           let _=set_wm_state(&conn,target,wm_state,1);
           let _=set_atom_state(&conn,target,net_wm_state,net_wm_state_hidden,false);
         } else {
           let _=set_wm_state(&conn,target,wm_state,1);
           let _=set_atom_state(&conn,target,net_wm_state,net_wm_state_hidden,true);
         }
         arrange(&conn,root,&s);
       }
       else if e.type_==net_wm_state {
         // EWMH action: 0=remove, 1=add, 2=toggle. State atoms are adopted
         // before mapping and then updated through this client-message path.
         let action=data[0]; let first=data[1]; let second=data[2];
         for atom in [first,second].into_iter().filter(|a| *a!=0) {
           if atom==net_wm_state_maximized_horz || atom==net_wm_state_maximized_vert {
             let mut s=state.lock().unwrap();
             let Some(client)=s.clients.get(&e.window) else { continue; };
             if client.fullscreen { continue; }
             let old_h=client.maximized_horz;
             let old_v=client.maximized_vert;
             let was_floating=client.floating;
             let monitor_name=client.monitor.clone();
             let current=conn.get_geometry(e.window).ok().and_then(|cookie|cookie.reply().ok());
             let current_rect=current.as_ref().map(|g|mrx_core::layout::Rect{x:g.x as i32,y:g.y as i32,width:g.width as u32,height:g.height as u32})
                 .unwrap_or(mrx_core::layout::Rect{x:client.x,y:client.y,width:client.width,height:client.height});
             let area=s.monitors.iter().find(|m|m.name==monitor_name)
                 .map(|m|monitor_workarea(m,&s.docks,s.root_width,s.root_height))
                 .unwrap_or(mrx_core::layout::Rect{x:0,y:0,width:s.root_width.max(1),height:s.root_height.max(1)});
             let requested=if atom==net_wm_state_maximized_horz { old_h } else { old_v };
             let enable=match action { 0=>false, 1=>true, 2=>!requested, _=>requested };
             let (next_h,next_v)=if atom==net_wm_state_maximized_horz {(enable,old_v)}else{(old_h,enable)};
             if next_h || next_v {
               s.restore_geometry.entry(e.window).or_insert((current_rect,was_floating));
               let (original,_)=s.restore_geometry.get(&e.window).cloned().unwrap_or((current_rect,was_floating));
               let target=maximized_rect(original,area,next_h,next_v);
               if let Some(window)=s.clients.get_mut(&e.window) {
                 window.maximized_horz=next_h; window.maximized_vert=next_v;
                 window.floating=true; window.x=target.x; window.y=target.y;
                 window.width=target.width; window.height=target.height;
               }
               let _=conn.configure_window(e.window,&ConfigureWindowAux::new().x(target.x).y(target.y).width(target.width).height(target.height));
             } else {
               let (original,original_floating)=s.restore_geometry.remove(&e.window).unwrap_or((current_rect,was_floating));
               if let Some(window)=s.clients.get_mut(&e.window) {
                 window.maximized_horz=false; window.maximized_vert=false;
                 window.floating=original_floating; window.x=original.x; window.y=original.y;
                 window.width=original.width; window.height=original.height;
               }
               let _=conn.configure_window(e.window,&ConfigureWindowAux::new().x(original.x).y(original.y).width(original.width).height(original.height).border_width(s.settings.appearance.border_width));
             }
             let _=set_atom_state(&conn,e.window,net_wm_state,net_wm_state_maximized_horz,next_h);
             let _=set_atom_state(&conn,e.window,net_wm_state,net_wm_state_maximized_vert,next_v);
             arrange(&conn,root,&s);
           }
           else if atom==net_wm_state_above || atom==net_wm_state_below {
             let mut s=state.lock().unwrap();
             let Some(client)=s.clients.get(&e.window) else { continue; };
             let old_above=client.above;
             let old_below=client.below;
             let requested=if atom==net_wm_state_above {old_above}else{old_below};
             let enable=match action {0=>false,1=>true,2=>!requested,_=>requested};
             let (next_above,next_below)=if atom==net_wm_state_above {
               (enable,if enable {false}else{old_below})
             } else {
               (if enable {false}else{old_above},enable)
             };
             if let Some(window)=s.clients.get_mut(&e.window) {
               window.above=next_above;
               window.below=next_below;
             }
             let _=set_atom_state(&conn,e.window,net_wm_state,net_wm_state_above,next_above);
             let _=set_atom_state(&conn,e.window,net_wm_state,net_wm_state_below,next_below);
             if next_above {
               let _=conn.configure_window(e.window,&ConfigureWindowAux::new().stack_mode(xproto::StackMode::ABOVE));
             } else if next_below {
               let _=conn.configure_window(e.window,&ConfigureWindowAux::new().stack_mode(xproto::StackMode::BELOW));
             }
             restack_managed_clients(&conn,&s);
             let _=publish_client_lists(&conn,root,clientlist,clientlist_stacking,&s);
           }
           else if atom==net_wm_state_modal {
             let mut s=state.lock().unwrap();
             let old=s.clients.get(&e.window).is_some_and(|client|client.modal);
             let enable=match action {0=>false,1=>true,2=>!old,_=>old};
             if let Some(client)=s.clients.get_mut(&e.window) { client.modal=enable; }
             let _=set_atom_state(&conn,e.window,net_wm_state,net_wm_state_modal,enable);
             if enable {
               for client in s.clients.values_mut(){client.focused=false;}
               let target=modal_focus_target(&s,e.window);
               if let Some(client)=s.clients.get_mut(&target){client.focused=true;}
               if let Err(error)=focus_client(&conn,target,wm_protocols,wm_take_focus){warn!(window=target,%error,"modal dialog focus negotiation failed");}
               let _=conn.change_property32(xproto::PropMode::REPLACE,root,net_active_window,AtomEnum::WINDOW,&[target]);
             } else {
               focus_most_recent(&conn,root,&mut s,wm_protocols,wm_take_focus,net_active_window);
             }
           }
           else if atom==net_wm_state_hidden {
             let mut s=state.lock().unwrap();
             let Some(client)=s.clients.get(&e.window) else { continue; };
             if client.dock { continue; }
             let was_minimized=client.minimized;
             let minimize=match action { 0=>false, 1=>true, 2=>!was_minimized, _=>was_minimized };
             let visible=client.workspace==active_workspace_for(&s,&client.monitor);
             if minimize!=was_minimized {
               if let Some(window)=s.clients.get_mut(&e.window) {
                 window.minimized=minimize;
                 if minimize { window.focused=false; }
               }
               if minimize {
                 let _=conn.unmap_window(e.window);
                 let _=set_wm_state(&conn,e.window,wm_state,3);
                 let _=set_atom_state(&conn,e.window,net_wm_state,net_wm_state_hidden,true);
                 focus_most_recent(&conn,root,&mut s,wm_protocols,wm_take_focus,net_active_window);
               } else if visible {
                 let _=conn.map_window(e.window);
                 let _=set_wm_state(&conn,e.window,wm_state,1);
                 let _=set_atom_state(&conn,e.window,net_wm_state,net_wm_state_hidden,false);
               } else {
                 let _=set_wm_state(&conn,e.window,wm_state,1);
                 let _=set_atom_state(&conn,e.window,net_wm_state,net_wm_state_hidden,true);
               }
               arrange(&conn,root,&s);
             }
           }
           else if atom==net_wm_state_fullscreen {
             let geometry=conn.get_geometry(e.window).ok().and_then(|cookie| cookie.reply().ok());
             let mut s=state.lock().unwrap();
             let monitor=s.clients.get(&e.window).and_then(|w| s.monitors.iter().find(|m| m.name == w.monitor)).or_else(|| s.monitors.iter().find(|m| m.focused)).or_else(|| s.monitors.first()).map(|m| (m.x,m.y,m.width,m.height)).unwrap_or((0,0,screen.width_in_pixels as u32,screen.height_in_pixels as u32));
             let border=s.settings.appearance.border_width;
             if let Some(w)=s.clients.get_mut(&e.window) {
               let enable=match action { 0=>false, 1=>true, 2=>!w.fullscreen, _=>w.fullscreen };
               if enable!=w.fullscreen {
                 if enable {
                   if let Some(g)=geometry.as_ref() { w.x=g.x as i32; w.y=g.y as i32; w.width=g.width as u32; w.height=g.height as u32; }
                 }
                 w.fullscreen=enable;
                 if enable {
                   let aux=ConfigureWindowAux::new().x(monitor.0).y(monitor.1).width(monitor.2).height(monitor.3).border_width(0);
                   let _=conn.configure_window(e.window,&aux);
                 } else {
                   let aux=ConfigureWindowAux::new().x(w.x).y(w.y).width(w.width).height(w.height).border_width(border);
                   let _=conn.configure_window(e.window,&aux);
                 }
                 let _=set_atom_state(&conn,e.window,net_wm_state,net_wm_state_fullscreen,enable);
                 restack_managed_clients(&conn,&s);
                 arrange(&conn,root,&s);
               }
             }
           }
         }
       }
       else if e.type_==desktop {
         let target=e.window;
         let requested=data[0];
         let mut s=state.lock().unwrap();
         if requested!=u32::MAX && requested<s.settings.workspace.count as u32 {
             let ws=requested+1;
             let is_dock=s.clients.get(&target).is_some_and(|w|w.dock);
             let minimized=s.clients.get(&target).is_some_and(|w|w.minimized);
             let visible=s.clients.get(&target).is_some_and(|w|!w.dock && !w.minimized && ws==active_workspace_for(&s,&w.monitor));
             if is_dock { continue; }
             if let Some(w)=s.clients.get_mut(&target){
                 w.workspace=ws;
                 let _=conn.change_property32(xproto::PropMode::REPLACE,target,desktop,AtomEnum::CARDINAL,&[requested]);
                 if minimized {let _=set_wm_state(&conn,target,wm_state,3);let _=set_atom_state(&conn,target,net_wm_state,net_wm_state_hidden,true);}
                 else if visible {let _=conn.map_window(target);let _=set_wm_state(&conn,target,wm_state,1);let _=set_atom_state(&conn,target,net_wm_state,net_wm_state_hidden,false);}
                 else {let _=conn.unmap_window(target);let _=set_wm_state(&conn,target,wm_state,1);let _=set_atom_state(&conn,target,net_wm_state,net_wm_state_hidden,true);}
             }
             arrange(&conn,root,&s);
         }
       }
       else if e.type_==current {
         let id=data[0].saturating_add(1);
         let mut s=state.lock().unwrap();
         if id>=1 && id<=s.settings.workspace.count as u32 {
             for w in s.clients.values_mut(){w.focused=false;}
             switch_focused_workspace(&conn,root,&mut s,id,None,workspace_atoms);
         }
       }
     }, Some(_) =>{}, None=>thread::sleep(Duration::from_millis(8)) }
   // Some X servers (including Xvfb) expose RandR 1.5 monitor-object changes
   // without delivering the expected notification to an already-running WM.
   // Reconcile at a low frequency as a safety net; only arrange when geometry
   // or monitor identity actually changes.
   if last_monitor_refresh.elapsed() >= Duration::from_millis(250) {
       let mut s = state.lock().unwrap();
       let width = s.root_width.min(u16::MAX as u32) as u16;
       let height = s.root_height.min(u16::MAX as u32) as u16;
       refresh_monitor_state(&conn, root, &mut s, width, height);
       last_monitor_refresh = std::time::Instant::now();
   }
   conn.flush()?; }
 // Remove WM-owned root properties before releasing SubstructureRedirect.
 for atom in [supported,wmcheck,current,count,clientlist,clientlist_stacking,net_active_window,net_workarea,net_desktop_names,net_viewport,net_geometry] {
     let _=conn.delete_property(root,atom);
 }
 let _=conn.destroy_window(check);
 conn.flush()?;
 let _=std::fs::remove_file(runtime_dir().join("mrx.sock"));
 info!("MRX X11 window manager stopped cleanly");
 Ok(())
}
fn refresh_monitor_state(conn: &RustConnection, root: Window, state: &mut State, width: u16, height: u16) {
    state.root_width = width as u32; state.root_height = height as u32;
    let discovered = discover_monitors(conn, root, width, height);
    let changed = discovered.len() != state.monitors.len()
        || discovered.iter().zip(&state.monitors).any(|(a,b)| a.name != b.name
            || a.x != b.x || a.y != b.y || a.width != b.width || a.height != b.height);
    if !changed { return; }
    let old_focus=focused_monitor_name(state);
    let old_active=state.active_by_monitor.clone();
    let mut next=discovered;
    let mut active=BTreeMap::new();
    for monitor in &mut next {
        let id=old_active.get(&monitor.name).copied().unwrap_or(1)
            .clamp(1,state.settings.workspace.count.max(1) as u32);
        monitor.active_workspace=id;
        active.insert(monitor.name.clone(),id);
    }
    let names:std::collections::BTreeSet<String>=next.iter().map(|m|m.name.clone()).collect();
    let fallback=next.iter().find(|m|m.name==old_focus).or_else(||next.iter().find(|m|m.focused))
        .or_else(||next.first()).cloned().unwrap_or_default();
    for window in state.clients.values_mut() {
        if !names.contains(&window.monitor) {
            window.monitor=fallback.name.clone();
            window.workspace=active.get(&fallback.name).copied().unwrap_or(1);
            if window.floating {
                let max_x=fallback.x+fallback.width.saturating_sub(window.width) as i32;
                let max_y=fallback.y+fallback.height.saturating_sub(window.height) as i32;
                window.x=window.x.clamp(fallback.x,max_x.max(fallback.x));
                window.y=window.y.clamp(fallback.y,max_y.max(fallback.y));
                let _=conn.configure_window(window.id,&ConfigureWindowAux::new().x(window.x).y(window.y));
            }
        }
    }
    state.monitors=next;
    state.active_by_monitor=active;
    let focus=if names.contains(&old_focus){old_focus}else{fallback.name.clone()};
    set_focused_monitor(state,&focus);
    info!(count=state.monitors.len(),names=?state.monitors.iter().map(|m|m.name.as_str()).collect::<Vec<_>>(),"RandR monitor topology refreshed");
    arrange(conn,root,state);
}

fn switch_monitor_workspace(
    conn: &RustConnection, root: Window, state: &mut State, monitor: &str,
    requested: u32, atoms: WorkspaceAtoms,
) {
    let count = state.settings.workspace.count.max(1) as u32;
    if !state.monitors.iter().any(|m| m.name == monitor) { return; }
    let id = requested.clamp(1, count);
    state.active_by_monitor.insert(monitor.to_owned(), id);
    set_focused_monitor(state, monitor);
    for window in state.clients.values().filter(|w| w.monitor == monitor && !w.dock) {
        if window.minimized { continue; }
        if window.workspace == id {
            let _ = conn.map_window(window.id);
            let _ = set_wm_state(conn, window.id, atoms.wm_state, 1);
            let _ = set_atom_state(conn, window.id, atoms.net_wm_state, atoms.net_wm_state_hidden, false);
        } else {
            let _ = conn.unmap_window(window.id);
            let _ = set_wm_state(conn, window.id, atoms.wm_state, 1);
            let _ = set_atom_state(conn, window.id, atoms.net_wm_state, atoms.net_wm_state_hidden, true);
        }
    }
    let _ = conn.delete_property(root, atoms.net_active_window);
    let _ = conn.change_property32(xproto::PropMode::REPLACE, root, atoms.current, AtomEnum::CARDINAL, &[id - 1]);
    arrange(conn, root, state);
}

fn switch_focused_workspace(
    conn: &RustConnection, root: Window, state: &mut State, requested: u32,
    relative: Option<i32>, atoms: WorkspaceAtoms,
) {
    let monitor = focused_monitor_name(state);
    let max = state.settings.workspace.count.max(1) as u32;
    let active = active_workspace_for(state, &monitor);
    let id = if let Some(delta) = relative {
        let next = active as i32 + delta;
        if state.settings.workspace.wrap {
            ((next - 1).rem_euclid(max as i32) + 1) as u32
        } else { next.clamp(1, max as i32) as u32 }
    } else { requested.clamp(1, max) };
    switch_monitor_workspace(conn, root, state, &monitor, id, atoms);
}

fn arrange(conn: &RustConnection, _root: Window, s: &State) {
    use mrx_core::layout::{arrange as calculate, LayoutKind};
    let layout = LayoutKind::parse(&s.settings.window.layout).unwrap_or(LayoutKind::MasterStack);
    let fallback = s.monitors.iter().find(|m| m.focused).or_else(|| s.monitors.first()).cloned().unwrap_or_default();
    let monitors = if s.monitors.is_empty() { vec![fallback] } else { s.monitors.clone() };
    for monitor in &monitors {
        let visible: Vec<&WindowInfo> = s.clients.values()
            .filter(|w| w.workspace == active_workspace_for(s, &monitor.name) && !w.dock && !w.minimized && !w.floating && !w.fullscreen && !w.maximized_horz && !w.maximized_vert
                && (w.monitor == monitor.name || (w.monitor.is_empty() && monitor.focused)))
            .collect();
        if visible.is_empty() { continue; }
        let area = monitor_workarea(monitor,&s.docks,s.root_width,s.root_height);
        let rects = calculate(
            area,
            visible.len(), layout, s.settings.window.master_ratio, s.settings.appearance.gaps,
        );
        for (window, rect) in visible.iter().zip(rects.iter()) {
            let constrained = constrain_rect(*rect, s.size_hints.get(&window.id));
            let aux = ConfigureWindowAux::new()
                .x(constrained.x).y(constrained.y).width(constrained.width).height(constrained.height)
                .border_width(s.settings.appearance.border_width);
            if let Err(error) = conn.configure_window(window.id, &aux) {
                warn!(window = window.id, %error, "failed to apply window geometry");
            }
        }
    }
}


/// Stable managed-client order with protocol layer precedence:
/// BELOW < normal < ABOVE < dock < fullscreen. Within each layer, retain
/// the WM's client insertion order.
fn stacking_order(state: &State) -> Vec<u32> {
    let layer = |id: &u32| -> u8 {
        let Some(window) = state.clients.get(id) else { return 1; };
        if window.fullscreen { 4 }
        else if window.dock { 3 }
        else if window.above { 2 }
        else if window.below { 0 }
        else { 1 }
    };
    let mut ids: Vec<u32> = state.client_order.iter()
        .copied()
        .filter(|id| state.clients.contains_key(id))
        .collect();
    ids.sort_by_key(|id| layer(id));
    ids
}

/// Reconcile the X server's managed-client order with EWMH layer requests.
/// Sibling-relative operations preserve the first managed client's relation
/// to unmanaged/override-redirect windows while enforcing the managed order.
fn restack_managed_clients(conn: &RustConnection, state: &State) {
    let order = stacking_order(state);
    for pair in order.windows(2) {
        let below = pair[0];
        let above = pair[1];
        if let Err(error) = conn.configure_window(
            above,
            &ConfigureWindowAux::new().sibling(below).stack_mode(xproto::StackMode::ABOVE),
        ) {
            warn!(window=above,sibling=below,%error,"failed to reconcile managed window stacking");
        }
    }
}

fn apply_setting(s: &mut State, key: &str, value: serde_json::Value) -> Result<()> {
    // Apply to a candidate copy first. Invalid values must never poison the
    // in-memory runtime state if validation or persistence fails.
    let mut candidate = s.settings.clone();
    match key {
        "animation.workspace_duration_ms" => candidate.animation.workspace_duration_ms = value.as_u64().context("expected unsigned integer")?,
        "animation.enabled" => candidate.animation.enabled = value.as_bool().context("expected boolean")?,
        "appearance.gaps" => candidate.appearance.gaps = u32::try_from(value.as_u64().context("expected non-negative integer")?).context("gaps value is too large")?,
        "appearance.border_width" => candidate.appearance.border_width = u32::try_from(value.as_u64().context("expected non-negative integer")?).context("border width is too large")?,
        "appearance.panel_opacity" => candidate.appearance.panel_opacity = value.as_f64().context("expected number")? as f32,
        "appearance.theme" => candidate.appearance.theme = value.as_str().context("expected string")?.to_owned(),
        "appearance.accent" => candidate.appearance.accent = value.as_str().context("expected string")?.to_owned(),
        "window.layout" => candidate.window.layout = value.as_str().context("expected string")?.to_owned(),
        "window.focus_follows_pointer" => candidate.window.focus_follows_pointer = value.as_bool().context("expected boolean")?,
        "window.master_ratio" => candidate.window.master_ratio = value.as_f64().context("expected number")? as f32,
        _ => anyhow::bail!("unknown or unsupported setting key: {key}"),
    }
    candidate.validate()?;
    candidate.save_atomic()?;
    s.settings = candidate;
    Ok(())
}


#[cfg(test)]
mod monitor_tests {
    use super::*;

    fn monitors() -> Vec<MonitorInfo> {
        vec![
            MonitorInfo { name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080, focused: true, active_workspace: 1 },
            MonitorInfo { name: "HDMI-1".into(), x: 1920, y: 0, width: 2560, height: 1440, focused: false, active_workspace: 1 },
            MonitorInfo { name: "DP-2".into(), x: -1280, y: 0, width: 1280, height: 1024, focused: false, active_workspace: 1 },
        ]
    }

    #[test]
    fn window_center_selects_the_monitor_it_occupies() {
        let monitors = monitors();
        assert_eq!(monitor_for_geometry(&monitors, 2100, 100, 800, 600), "HDMI-1");
        assert_eq!(monitor_for_geometry(&monitors, -1200, 100, 800, 600), "DP-2");
    }

    #[test]
    fn offscreen_windows_fall_back_to_the_focused_monitor() {
        assert_eq!(monitor_for_geometry(&monitors(), 6000, 4000, 800, 600), "DP-1");
    }

    #[test]
    fn normal_size_hints_apply_minimum_and_maximum_constraints() {
        let hints = WmSizeHints { min_size: Some((400, 300)), max_size: Some((900, 700)), ..Default::default() };
        let rect = constrain_rect(mrx_core::layout::Rect { x: 0, y: 0, width: 1200, height: 900 }, Some(&hints));
        assert_eq!((rect.width, rect.height), (900, 700));
    }

    #[test]
    fn normal_size_hints_apply_resize_increments() {
        let hints = WmSizeHints {
            min_size: Some((100, 80)),
            size_increment: Some((64, 32)),
            base_size: Some((0, 0)),
            ..Default::default()
        };
        let rect = constrain_rect(mrx_core::layout::Rect { x: 0, y: 0, width: 850, height: 650 }, Some(&hints));
        assert_eq!((rect.width, rect.height), (832, 640));
    }

    #[test]
    fn fixed_size_hints_are_detected_for_floating_policy() {
        let hints = WmSizeHints { min_size: Some((320, 200)), max_size: Some((320, 200)), ..Default::default() };
        assert!(has_fixed_size(Some(&hints)));
        assert!(!has_fixed_size(None));
    }

    #[test]
    fn normal_size_hints_apply_aspect_ratio_bounds() {
        use x11rb::properties::AspectRatio;
        let min_aspect = AspectRatio::new(4, 3);
        let max_aspect = AspectRatio::new(16, 9);
        let hints = WmSizeHints { aspect: Some((min_aspect, max_aspect)), ..Default::default() };
        let narrow = constrain_rect(mrx_core::layout::Rect { x: 0, y: 0, width: 400, height: 500 }, Some(&hints));
        assert_eq!((narrow.width, narrow.height), (400, 300));
        let wide = constrain_rect(mrx_core::layout::Rect { x: 0, y: 0, width: 1000, height: 400 }, Some(&hints));
        assert_eq!((wide.width, wide.height), (711, 400));
    }

    #[test]
    fn slow_subscription_socket_write_records_backpressure_and_latency() {
        let (mut writer,_reader)=UnixStream::pair().unwrap();
        writer.set_write_timeout(Some(Duration::from_millis(10))).unwrap();
        let payload=vec![0u8;4*1024*1024];
        let started=Instant::now();
        let result=writer.write_all(&payload);
        assert!(result.as_ref().is_err_and(|error|matches!(error.kind(),std::io::ErrorKind::TimedOut|std::io::ErrorKind::WouldBlock)));
        let elapsed=started.elapsed();
        let error=result.err().map(|error|error.kind());
        let mut state=State::default();
        record_subscription_write(&mut state,0,elapsed,error);
        assert_eq!(state.subscription_write_timeouts,1);
        assert_eq!(state.subscription_slow_clients_dropped,1);
        assert!(state.subscription_max_write_latency_ms>0);
    }

    #[test]
    fn modal_focus_redirects_to_deepest_visible_transient_descendant() {
        let mut state = State { monitors: monitors(), focused_monitor: "DP-1".into(), ..Default::default() };
        state.client_order = vec![10, 11, 12];
        state.clients.insert(10, WindowInfo { id: 10, workspace: 1, monitor: "DP-1".into(), ..Default::default() });
        state.clients.insert(11, WindowInfo { id: 11, workspace: 1, monitor: "DP-1".into(), transient_for: 10, modal: true, floating: true, ..Default::default() });
        state.clients.insert(12, WindowInfo { id: 12, workspace: 1, monitor: "DP-1".into(), transient_for: 11, modal: true, floating: true, ..Default::default() });
        assert_eq!(modal_focus_target(&state, 10), 12);
        assert_eq!(preferred_focus_client(&state), Some(12));
        state.clients.get_mut(&12).unwrap().minimized = true;
        assert_eq!(modal_focus_target(&state, 10), 11);
        assert_eq!(preferred_focus_client(&state), Some(11));
    }

    #[test]
    fn managed_stacking_order_respects_protocol_layers_and_stability() {
        let mut state = State { client_order: vec![1, 2, 3, 4, 5, 6], ..Default::default() };
        state.clients.insert(1, WindowInfo { id: 1, ..Default::default() });
        state.clients.insert(2, WindowInfo { id: 2, above: true, ..Default::default() });
        state.clients.insert(3, WindowInfo { id: 3, below: true, ..Default::default() });
        state.clients.insert(4, WindowInfo { id: 4, dock: true, ..Default::default() });
        state.clients.insert(5, WindowInfo { id: 5, fullscreen: true, ..Default::default() });
        state.clients.insert(6, WindowInfo { id: 6, ..Default::default() });
        assert_eq!(stacking_order(&state), vec![3, 1, 6, 2, 4, 5]);
    }

    #[test]
    fn synthetic_screen_monitor_is_removed_when_real_outputs_exist() {
        let mut list = vec![
            MonitorInfo { name: "screen".into(), x: 0, y: 0, width: 1920, height: 1080, focused: true, active_workspace: 1 },
            MonitorInfo { name: "DP-1".into(), x: 0, y: 0, width: 960, height: 1080, focused: false, active_workspace: 1 },
            MonitorInfo { name: "HDMI-1".into(), x: 960, y: 0, width: 960, height: 1080, focused: false, active_workspace: 1 },
        ];
        discard_synthetic_screen_monitor(&mut list);
        assert_eq!(list.iter().map(|m|m.name.as_str()).collect::<Vec<_>>(), vec!["DP-1", "HDMI-1"]);
    }

    #[test]
    fn synthetic_screen_monitor_is_kept_as_the_only_fallback() {
        let mut list = vec![
            MonitorInfo { name: "screen".into(), x: 0, y: 0, width: 1920, height: 1080, focused: true, active_workspace: 1 },
        ];
        discard_synthetic_screen_monitor(&mut list);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "screen");
    }

    #[test]
    fn monitor_workspace_ids_are_independent() {
        let mut state = State { settings: Settings::default(), monitors: monitors(), ..Default::default() };
        state.active_by_monitor.insert("DP-1".into(), 2);
        state.active_by_monitor.insert("HDMI-1".into(), 3);
        set_focused_monitor(&mut state, "HDMI-1");
        assert_eq!(active_workspace_for(&state, "DP-1"), 2);
        assert_eq!(active_workspace_for(&state, "HDMI-1"), 3);
        assert_eq!(state.active, 3);
    }

    #[test]
    fn snapshot_emits_monitor_scoped_workspace_instances() {
        let mut state = State { settings: Settings::default(), monitors: monitors(), ..Default::default() };
        state.active_by_monitor.insert("DP-1".into(), 2);
        state.active_by_monitor.insert("HDMI-1".into(), 3);
        state.clients.insert(10, WindowInfo { id: 10, workspace: 2, monitor: "DP-1".into(), ..Default::default() });
        state.clients.insert(11, WindowInfo { id: 11, workspace: 3, monitor: "HDMI-1".into(), ..Default::default() });
        set_focused_monitor(&mut state, "DP-1");
        let initial_snapshot = snapshot(&state);
        assert_eq!(initial_snapshot.workspaces.len(), state.monitors.len() * state.settings.workspace.count);
        assert!(initial_snapshot.workspaces.iter().any(|w| w.monitor == "DP-1" && w.id == 2 && w.active && w.windows == 1));
        assert!(initial_snapshot.workspaces.iter().any(|w| w.monitor == "HDMI-1" && w.id == 3 && w.active && w.windows == 1));
        state.clients.insert(12, WindowInfo { id: 12, workspace: 2, monitor: "DP-1".into(), dock: true, ..Default::default() });
        let dock_snapshot = snapshot(&state);
        assert!(dock_snapshot.windows.iter().any(|window| window.id == 12 && window.dock));
        assert!(dock_snapshot.workspaces.iter().any(|workspace| workspace.monitor == "DP-1" && workspace.id == 2 && workspace.windows == 1));
        assert_eq!(initial_snapshot.active_workspace, 2);
    }

    #[test]
    fn dock_strut_reserves_top_edge_from_workarea() {
        let monitor=MonitorInfo{name:"DP-1".into(),x:0,y:0,width:1920,height:1080,focused:true,active_workspace:1};
        let strut=DockStrut{top:48,top_start_x:0,top_end_x:1919,..Default::default()};
        let docks=BTreeMap::from([(99,strut)]);
        let area=monitor_workarea(&monitor,&docks,1920,1080);
        assert_eq!((area.x,area.y,area.width,area.height),(0,48,1920,1032));
    }

    #[test]
    fn right_and_bottom_struts_use_root_edge_distances() {
        let monitor=MonitorInfo{name:"DP-2".into(),x:1280,y:0,width:640,height:1080,focused:false,active_workspace:1};
        let strut=DockStrut{right:100,bottom:80,right_start_y:0,right_end_y:1079,bottom_start_x:0,bottom_end_x:1919,..Default::default()};
        let area=monitor_workarea(&monitor,&BTreeMap::from([(2,strut)]),1920,1080);
        assert_eq!((area.x,area.y,area.width,area.height),(1280,0,540,1000));
    }

    #[test]
    fn right_strut_does_not_shrink_non_intersecting_left_monitor() {
        let monitor=MonitorInfo{name:"DP-1".into(),x:0,y:0,width:1280,height:720,focused:true,active_workspace:1};
        let strut=DockStrut{right:100,right_start_y:0,right_end_y:719,..Default::default()};
        let area=monitor_workarea(&monitor,&BTreeMap::from([(3,strut)]),1920,1080);
        assert_eq!((area.x,area.y,area.width,area.height),(0,0,1280,720));
    }

    #[test]
    fn dock_strut_is_ignored_when_partial_range_misses_monitor() {
        let monitor=MonitorInfo{name:"HDMI-1".into(),x:1920,y:0,width:1280,height:720,focused:false,active_workspace:1};
        let strut=DockStrut{top:50,top_start_x:0,top_end_x:1000,..Default::default()};
        let docks=BTreeMap::from([(100,strut)]);
        let area=monitor_workarea(&monitor,&docks,3200,1080);
        assert_eq!((area.x,area.y,area.width,area.height),(1920,0,1280,720));
    }

    #[test]
    fn oversized_dock_struts_never_make_empty_workareas() {
        let monitor=MonitorInfo{name:"DP-1".into(),x:0,y:0,width:100,height:80,focused:true,active_workspace:1};
        let strut=DockStrut{left:500,top:500,right:500,bottom:500,..Default::default()};
        let area=monitor_workarea(&monitor,&BTreeMap::from([(1,strut)]),100,80);
        assert!(area.width>=1 && area.height>=1);
    }

    #[test]
    fn ewmh_maximize_axes_preserve_unrequested_geometry() {
        let original=mrx_core::layout::Rect{x:120,y:90,width:800,height:600};
        let workarea=mrx_core::layout::Rect{x:0,y:48,width:1920,height:1032};
        let horizontal=maximized_rect(original,workarea,true,false);
        assert_eq!((horizontal.x,horizontal.y,horizontal.width,horizontal.height),(0,90,1920,600));
        let vertical=maximized_rect(original,workarea,false,true);
        assert_eq!((vertical.x,vertical.y,vertical.width,vertical.height),(120,48,800,1032));
        let both=maximized_rect(original,workarea,true,true);
        assert_eq!(both,workarea);
        let restored=maximized_rect(original,workarea,false,false);
        assert_eq!(restored,original);
    }

    #[test]
    fn empty_monitor_registry_is_safe() {
        assert_eq!(monitor_for_geometry(&[], 0, 0, 800, 600), "");
    }
}
