use anyhow::{Context, Result};
use mrx_core::{Request, RequestEnvelope, Response, Settings, Snapshot, WindowInfo, WorkspaceInfo, MonitorInfo, IPC_PROTOCOL_VERSION};
use std::{collections::BTreeMap, io::{BufRead, BufReader, Read, Write}, os::unix::{fs::{FileTypeExt, PermissionsExt}, net::{UnixListener, UnixStream}}, path::PathBuf, sync::{atomic::{AtomicUsize, Ordering}, Arc, Mutex, mpsc}, thread, time::Duration};
use tracing::{info, warn};
use x11rb::{connection::{Connection, RequestConnection}, properties::{WmHints, WmSizeHints}, protocol::{randr::{self, ConnectionExt as RandrConnectionExt, NotifyMask}, xproto::{self, ConnectionExt as XprotoConnectionExt, AtomEnum, ChangeWindowAttributesAux, ConfigureWindowAux, EventMask, GrabMode, InputFocus, ModMask, Window}, Event}, rust_connection::RustConnection, wrapper::ConnectionExt as WrapperConnectionExt, CURRENT_TIME};

#[derive(Default)] struct State { active:u32, clients:BTreeMap<u32,WindowInfo>, client_order:Vec<u32>, settings:Settings, monitors:Vec<MonitorInfo>, size_hints:BTreeMap<u32,WmSizeHints> }
#[derive(Debug)] enum Action { Switch(u32), Relative(i32), Focus(u32), Close(u32), Move(u32,u32), Set(String,serde_json::Value), Reload }
fn runtime_dir()->PathBuf { std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(||PathBuf::from(format!("/tmp/mrx-{}",std::env::var("USER").unwrap_or_else(|_|"user".into())))) }
fn snapshot(s:&State)->Snapshot { let workspaces=(1..=s.settings.workspace.count as u32).map(|id| WorkspaceInfo{id,name:s.settings.workspace.names.get((id-1) as usize).cloned().unwrap_or_else(||id.to_string()),active:id==s.active,windows:s.clients.values().filter(|w|w.workspace==id).count()}).collect(); Snapshot{protocol_version:mrx_core::IPC_PROTOCOL_VERSION,compositor:"mrx-x11".into(),active_workspace:s.active,workspaces,windows:s.clients.values().cloned().collect(),settings:s.settings.clone(),monitors:s.monitors.clone()} }
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

fn handle_ipc_connection(mut stream: UnixStream, state: &Arc<Mutex<State>>, tx: &mpsc::Sender<Action>) -> Result<()> {
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
    let response = match request {
        Err(error) => Response::err(format!("invalid request envelope: {error}")),
        Ok(envelope) => {
            let request_id = envelope.request_id;
            let mut response = if let Err(error) = envelope.validate() {
                Response::err(error.to_string())
            } else {
                match envelope.request {
                    Request::Ping => Response::ok("pong"),
                    Request::Snapshot => {
                        let mut response = Response::ok("snapshot");
                        response.snapshot = Some(snapshot(&state.lock().unwrap()));
                        response
                    }
                    Request::ReloadSettings => {
                        tx.send(Action::Reload).context("WM event loop is unavailable")?;
                        Response::ok("settings reload queued")
                    }
                    Request::SetSetting { key, value } => {
                        tx.send(Action::Set(key, value)).context("WM event loop is unavailable")?;
                        Response::ok("setting update queued")
                    }
                    Request::SwitchWorkspace { id } => {
                        tx.send(Action::Switch(id)).context("WM event loop is unavailable")?;
                        Response::ok("workspace switch queued")
                    }
                    Request::SwitchRelative { direction } => {
                        let delta = if direction == "next" { 1 } else { -1 };
                        tx.send(Action::Relative(delta)).context("WM event loop is unavailable")?;
                        Response::ok("workspace switch queued")
                    }
                    Request::FocusWindow { id } => {
                        tx.send(Action::Focus(id)).context("WM event loop is unavailable")?;
                        Response::ok("focus queued")
                    }
                    Request::CloseWindow { id } => {
                        tx.send(Action::Close(id)).context("WM event loop is unavailable")?;
                        Response::ok("close queued")
                    }
                    Request::MoveWindow { id, workspace } => {
                        tx.send(Action::Move(id, workspace)).context("WM event loop is unavailable")?;
                        Response::ok("move queued")
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
                                    if let Err(error) = handle_ipc_connection(stream, &client_state, &client_tx) {
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
                        focused: monitor.primary,
                    });
                }
            }
            Err(error) => warn!(%error, "RandR GetMonitors reply failed; using active CRTC outputs"),
            },
            Err(error) => warn!(%error, "RandR GetMonitors request failed; using active CRTC outputs"),
        }
    }

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
                    focused: false,
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
            width: width as u32, height: height as u32, focused: true,
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

/// Restore focus to the most recently managed client on the active workspace.
fn focus_most_recent(
    conn: &RustConnection,
    root: Window,
    state: &mut State,
    wm_protocols: u32,
    wm_take_focus: u32,
    net_active_window: u32,
) {
    let candidate = state.client_order.iter().rev().find(|id| {
        state.clients.get(*id).is_some_and(|window| window.workspace == state.active)
    }).copied();
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

/// Publish EWMH client lists with their required ordering semantics.
/// `_NET_CLIENT_LIST` follows initial map order; `_NET_CLIENT_LIST_STACKING`
/// follows the X server's bottom-to-top child stacking order.
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

fn is_transient(conn: &RustConnection, window: Window) -> Result<bool> {
    let reply = conn.get_property(false, window, AtomEnum::WM_TRANSIENT_FOR, AtomEnum::WINDOW, 0, 1)?.reply()?;
    Ok(reply.value32().and_then(|mut values| values.next()).is_some())
}
fn main()->Result<()> { tracing_subscriber::fmt().with_env_filter("info").init(); let (conn,screen_num)=x11rb::connect(None).context("cannot connect to X11 display")?; let screen=&conn.setup().roots[screen_num]; let root=screen.root; conn.change_window_attributes(root,&ChangeWindowAttributesAux::new().event_mask(EventMask::SUBSTRUCTURE_REDIRECT|EventMask::SUBSTRUCTURE_NOTIFY|EventMask::PROPERTY_CHANGE|EventMask::BUTTON_PRESS)).context("another window manager may already own SubstructureRedirect")?.check()?;
 let supported=intern(&conn,"_NET_SUPPORTED")?; let wmcheck=intern(&conn,"_NET_SUPPORTING_WM_CHECK")?; let name=intern(&conn,"_NET_WM_NAME")?; let utf8=intern(&conn,"UTF8_STRING")?; let current=intern(&conn,"_NET_CURRENT_DESKTOP")?; let count=intern(&conn,"_NET_NUMBER_OF_DESKTOPS")?; let clientlist=intern(&conn,"_NET_CLIENT_LIST")?; let clientlist_stacking=intern(&conn,"_NET_CLIENT_LIST_STACKING")?; let desktop=intern(&conn,"_NET_WM_DESKTOP")?; let wm_protocols=intern(&conn,"WM_PROTOCOLS")?; let wm_hints=intern(&conn,"WM_HINTS")?; let wm_normal_hints=intern(&conn,"WM_NORMAL_HINTS")?; let wm_delete=intern(&conn,"WM_DELETE_WINDOW")?; let wm_take_focus=intern(&conn,"WM_TAKE_FOCUS")?; let wm_state=intern(&conn,"WM_STATE")?; let net_close=intern(&conn,"_NET_CLOSE_WINDOW")?; let net_active=intern(&conn,"_NET_ACTIVE_WINDOW")?; let net_wm_state=intern(&conn,"_NET_WM_STATE")?; let net_wm_state_fullscreen=intern(&conn,"_NET_WM_STATE_FULLSCREEN")?; let net_wm_state_hidden=intern(&conn,"_NET_WM_STATE_HIDDEN")?; let net_desktop_names=intern(&conn,"_NET_DESKTOP_NAMES")?; let net_workarea=intern(&conn,"_NET_WORKAREA")?; let net_viewport=intern(&conn,"_NET_DESKTOP_VIEWPORT")?; let net_geometry=intern(&conn,"_NET_DESKTOP_GEOMETRY")?; let net_active_window=intern(&conn,"_NET_ACTIVE_WINDOW")?;
 let atoms=[net_active,net_close,desktop,current,count,clientlist,clientlist_stacking,net_wm_state,net_wm_state_fullscreen,net_wm_state_hidden,net_desktop_names,net_workarea,net_viewport,net_geometry]; conn.change_property32(xproto::PropMode::REPLACE,root,supported,AtomEnum::ATOM,&atoms)?; conn.change_property8(xproto::PropMode::REPLACE,root,name,utf8,b"MRX Rust Desktop")?; let check=conn.generate_id()?; conn.create_window(x11rb::COPY_DEPTH_FROM_PARENT,check,root,0,0,1,1,0,xproto::WindowClass::INPUT_OUTPUT,0,&xproto::CreateWindowAux::new())?; conn.change_property32(xproto::PropMode::REPLACE,root,wmcheck,AtomEnum::WINDOW,&[check])?; conn.change_property32(xproto::PropMode::REPLACE,check,wmcheck,AtomEnum::WINDOW,&[check])?; conn.change_property8(xproto::PropMode::REPLACE,check,name,utf8,b"MRX Rust Desktop")?;
 let monitors = discover_monitors(&conn, root, screen.width_in_pixels, screen.height_in_pixels);
 let state=Arc::new(Mutex::new(State{active:1,settings:Settings::load().unwrap_or_default(),monitors,..Default::default()})); let workspace_count=state.lock().unwrap().settings.workspace.count as u32;
 conn.change_property32(xproto::PropMode::REPLACE,root,count,AtomEnum::CARDINAL,&[workspace_count])?;
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
 conn.change_property32(xproto::PropMode::REPLACE,root,net_workarea,AtomEnum::CARDINAL,&(0..workspace_count).flat_map(|_| [0u32,0,screen.width_in_pixels as u32,screen.height_in_pixels as u32]).collect::<Vec<_>>())?;
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
     if attributes.override_redirect || attributes.map_state==xproto::MapState::UNMAPPED { continue; }
     let geometry=match conn.get_geometry(window)?.reply() { Ok(value)=>value, Err(_)=>continue };
     let title=conn.get_property(false,window,AtomEnum::WM_NAME,AtomEnum::STRING,0,1024)?.reply()?.value;
     let size_hints = read_size_hints(&conn, window);
     let floating = is_transient(&conn,window).unwrap_or(false) || has_fixed_size(size_hints.as_ref());
     let title=String::from_utf8_lossy(&title).into_owned();
     let _=conn.change_window_attributes(window,&ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE));
     s.client_order.push(window);
     if let Some(hints) = size_hints { s.size_hints.insert(window, hints); }
     let assigned_monitor=monitor_for_geometry(&s.monitors,geometry.x as i32,geometry.y as i32,geometry.width as u32,geometry.height as u32);s.clients.insert(window,WindowInfo{id:window,title,class:String::new(),workspace:1,monitor:assigned_monitor,x:geometry.x as i32,y:geometry.y as i32,width:geometry.width as u32,height:geometry.height as u32,focused:false,floating,urgent:read_urgent_hint(&conn,window),..Default::default()});
     let _=set_wm_state(&conn,window,wm_state,1);
     let _=conn.change_property32(xproto::PropMode::REPLACE,window,desktop,AtomEnum::CARDINAL,&[0]);
   }
   if let Some(topmost)=s.client_order.last().copied() {
     if let Some(client)=s.clients.get_mut(&topmost) { client.focused=true; }
     let _=focus_client(&conn,topmost,wm_protocols,wm_take_focus);
     let _=conn.change_property32(xproto::PropMode::REPLACE,root,net_active_window,AtomEnum::WINDOW,&[topmost]);
   }
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
 loop { while let Ok(a)=rx.try_recv(){ let mut s=state.lock().unwrap(); match a { Action::Switch(id)=>{let id=id.clamp(1,s.settings.workspace.count as u32); s.active=id; for w in s.clients.values_mut(){w.focused=false;} let _=conn.delete_property(root,net_active_window); for w in s.clients.values(){if w.workspace==id{let _=conn.map_window(w.id);let _=set_wm_state(&conn,w.id,wm_state,1);let _=set_atom_state(&conn,w.id,net_wm_state,net_wm_state_hidden,false);}else{let _=conn.unmap_window(w.id);let _=set_wm_state(&conn,w.id,wm_state,1);let _=set_atom_state(&conn,w.id,net_wm_state,net_wm_state_hidden,true);}} let _=conn.change_property32(xproto::PropMode::REPLACE,root,current,AtomEnum::CARDINAL,&[id-1]);arrange(&conn,root,&s);}, Action::Relative(d)=>{let max=s.settings.workspace.count as i32;let mut id=s.active as i32+d;if s.settings.workspace.wrap{id=((id-1).rem_euclid(max))+1}else{id=id.clamp(1,max)};s.active=id as u32;for w in s.clients.values_mut(){w.focused=false;}let _=conn.delete_property(root,net_active_window);for w in s.clients.values(){if w.workspace==s.active{let _=conn.map_window(w.id);let _=set_wm_state(&conn,w.id,wm_state,1);let _=set_atom_state(&conn,w.id,net_wm_state,net_wm_state_hidden,false);}else{let _=conn.unmap_window(w.id);let _=set_wm_state(&conn,w.id,wm_state,1);let _=set_atom_state(&conn,w.id,net_wm_state,net_wm_state_hidden,true);}}let _=conn.change_property32(xproto::PropMode::REPLACE,root,current,AtomEnum::CARDINAL,&[s.active-1]);arrange(&conn,root,&s);}, Action::Focus(id)=>{if s.clients.get(&id).is_some_and(|w|w.workspace==s.active){for w in s.clients.values_mut(){w.focused=w.id==id;}if let Err(error)=focus_client(&conn,id,wm_protocols,wm_take_focus){warn!(window=id,%error,"ICCCM focus negotiation failed");}let _=conn.change_property32(xproto::PropMode::REPLACE,root,net_active_window,AtomEnum::WINDOW,&[id]);}}, Action::Close(id)=>{if let Err(e)=request_close(&conn,id,wm_protocols,wm_delete){warn!(window=id,error=%e,"close request failed");}}, Action::Move(id,ws)=>{if ws==0 || ws>s.settings.workspace.count as u32 { warn!(workspace=ws,"move rejected: invalid workspace"); continue; } if let Some(w)=s.clients.get_mut(&id){w.workspace=ws;let _=conn.change_property32(xproto::PropMode::REPLACE,id,desktop,AtomEnum::CARDINAL,&[ws-1]);if ws==s.active{let _=conn.map_window(id);let _=set_wm_state(&conn,id,wm_state,1);let _=set_atom_state(&conn,id,net_wm_state,net_wm_state_hidden,false);}else{let _=conn.unmap_window(id);let _=set_wm_state(&conn,id,wm_state,1);let _=set_atom_state(&conn,id,net_wm_state,net_wm_state_hidden,true);}}arrange(&conn,root,&s);}, Action::Set(k,v)=>{if let Err(e)=apply_setting(&mut s,&k,v){warn!(error=%e,"setting rejected")}else{for w in s.clients.values(){let _=conn.configure_window(w.id,&ConfigureWindowAux::new().border_width(s.settings.appearance.border_width));}arrange(&conn,root,&s);}}, Action::Reload=>{match Settings::load(){Ok(c)=>s.settings=c,Err(e)=>warn!(error=%e,"settings reload rejected; keeping previous settings")}} } }
   match conn.poll_for_event()? { Some(Event::RandrNotify(_)) | Some(Event::RandrScreenChangeNotify(_)) => {
        let mut s = state.lock().unwrap();
        refresh_monitor_state(&conn, root, &mut s, screen.width_in_pixels, screen.height_in_pixels);
    }, Some(Event::PropertyNotify(e))=>{
        if e.atom == wm_hints || e.atom == wm_normal_hints {
            let urgent = if e.atom == wm_hints { Some(read_urgent_hint(&conn, e.window)) } else { None };
            let size_hints = if e.atom == wm_normal_hints { Some(read_size_hints(&conn, e.window)) } else { None };
            let transient = is_transient(&conn, e.window).unwrap_or(false);
            let mut s = state.lock().unwrap();
            if let Some(urgent) = urgent {
                if let Some(window) = s.clients.get_mut(&e.window) { window.urgent = urgent; }
            }
            if let Some(size_hints) = size_hints {
                if let Some(hints) = size_hints.as_ref() { s.size_hints.insert(e.window, *hints); }
                else { s.size_hints.remove(&e.window); }
                let fixed = has_fixed_size(size_hints.as_ref());
                if let Some(window) = s.clients.get_mut(&e.window) { window.floating = transient || fixed; }
            }
            arrange(&conn, root, &s);
        }
     }, Some(Event::MapRequest(e))=>{
        info!(window=e.window,"received X11 MapRequest");
        let mut s=state.lock().unwrap();
        let ws=s.active;
        if let Err(error)=conn.map_window(e.window) { warn!(window=e.window,%error,"failed to map requested client"); continue; }
        let _=conn.configure_window(e.window,&ConfigureWindowAux::new().border_width(s.settings.appearance.border_width));
        let g=match conn.get_geometry(e.window) { Ok(cookie)=>match cookie.reply(){Ok(geometry)=>geometry,Err(error)=>{warn!(window=e.window,%error,"failed to query requested client geometry");continue;}},Err(error)=>{warn!(window=e.window,%error,"failed to request client geometry");continue;} };
        let size_hints = read_size_hints(&conn, e.window);
         let floating = is_transient(&conn,e.window).unwrap_or(false) || has_fixed_size(size_hints.as_ref());
        let title=conn.get_property(false,e.window,AtomEnum::WM_NAME,AtomEnum::STRING,0,1024).ok()
            .and_then(|cookie|cookie.reply().ok()).map(|reply|String::from_utf8_lossy(&reply.value).to_string()).unwrap_or_default();
        for client in s.clients.values_mut(){client.focused=false;}
        if !s.clients.contains_key(&e.window) { s.client_order.push(e.window); }
         if let Some(hints) = size_hints { s.size_hints.insert(e.window, hints); }
        let assigned_monitor=conn.query_pointer(root).ok().and_then(|cookie|cookie.reply().ok()).map(|pointer|monitor_for_geometry(&s.monitors,pointer.root_x as i32,pointer.root_y as i32,1,1)).unwrap_or_else(||monitor_for_geometry(&s.monitors,g.x as i32,g.y as i32,g.width as u32,g.height as u32));s.clients.insert(e.window,WindowInfo{id:e.window,title,class:String::new(),workspace:ws,monitor:assigned_monitor,x:g.x as i32,y:g.y as i32,width:g.width as u32,height:g.height as u32,focused:true,floating,urgent:read_urgent_hint(&conn,e.window),..Default::default()});
        let _=conn.change_window_attributes(e.window,&ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE));
        if let Err(error)=focus_client(&conn,e.window,wm_protocols,wm_take_focus){warn!(window=e.window,%error,"ICCCM focus negotiation failed");}
        let _=conn.change_property32(xproto::PropMode::REPLACE,root,net_active_window,AtomEnum::WINDOW,&[e.window]);
        let _=set_wm_state(&conn,e.window,wm_state,1);
        let _=conn.change_property32(xproto::PropMode::REPLACE,e.window,desktop,AtomEnum::CARDINAL,&[ws-1]);
        if let Err(error)=publish_client_lists(&conn,root,clientlist,clientlist_stacking,&s){warn!(%error,"failed to publish EWMH client lists");}
        arrange(&conn,root,&s);
    }, Some(Event::UnmapNotify(e))=>{let mut s=state.lock().unwrap();if s.clients.get(&e.window).is_some_and(|w|w.workspace==s.active){s.clients.remove(&e.window);s.size_hints.remove(&e.window);s.client_order.retain(|id|*id!=e.window);let _=set_wm_state(&conn,e.window,wm_state,0);if let Err(error)=publish_client_lists(&conn,root,clientlist,clientlist_stacking,&s){warn!(%error,"failed to publish EWMH client lists");}focus_most_recent(&conn,root,&mut s,wm_protocols,wm_take_focus,net_active_window);arrange(&conn,root,&s);}}, Some(Event::DestroyNotify(e))=>{let mut s=state.lock().unwrap();s.clients.remove(&e.window);s.size_hints.remove(&e.window);s.client_order.retain(|id|*id!=e.window);if let Err(error)=publish_client_lists(&conn,root,clientlist,clientlist_stacking,&s){warn!(%error,"failed to publish EWMH client lists");}focus_most_recent(&conn,root,&mut s,wm_protocols,wm_take_focus,net_active_window);arrange(&conn,root,&s);}, Some(Event::ConfigureRequest(e))=>{
       let managed = state.lock().unwrap().clients.get(&e.window).cloned();
       if let Some(window) = managed {
         if window.floating && !window.fullscreen {
           let aux=ConfigureWindowAux::new().x(e.x as i32).y(e.y as i32).width(e.width as u32).height(e.height as u32).border_width(e.border_width as u32).sibling(e.sibling).stack_mode(e.stack_mode);
           let _=conn.configure_window(e.window,&aux);
         } else { let s=state.lock().unwrap(); arrange(&conn,root,&s); }
       } else {
         let aux=ConfigureWindowAux::new().x(e.x as i32).y(e.y as i32).width(e.width as u32).height(e.height as u32).border_width(e.border_width as u32).sibling(e.sibling).stack_mode(e.stack_mode);
         let _=conn.configure_window(e.window,&aux);
       }
     }, Some(Event::KeyPress(e))=>{let key=e.detail; if (10..=18).contains(&key){let id=(key-9) as u32;let mut s=state.lock().unwrap();s.active=id;for w in s.clients.values_mut(){w.focused=false;}let _=conn.delete_property(root,net_active_window);for w in s.clients.values(){if w.workspace==id{let _=conn.map_window(w.id);let _=set_wm_state(&conn,w.id,wm_state,1);let _=set_atom_state(&conn,w.id,net_wm_state,net_wm_state_hidden,false);}else{let _=conn.unmap_window(w.id);let _=set_wm_state(&conn,w.id,wm_state,1);let _=set_atom_state(&conn,w.id,net_wm_state,net_wm_state_hidden,true);}}let _=conn.change_property32(xproto::PropMode::REPLACE,root,current,AtomEnum::CARDINAL,&[id-1]);arrange(&conn,root,&s);}else if key==113||key==114{let mut s=state.lock().unwrap();let max=s.settings.workspace.count as i32;let delta=if key==114{1}else{-1};let id=if s.settings.workspace.wrap{((s.active as i32-1+delta).rem_euclid(max)+1) as u32}else{(s.active as i32+delta).clamp(1,max) as u32};s.active=id;for w in s.clients.values_mut(){w.focused=false;}let _=conn.delete_property(root,net_active_window);for w in s.clients.values(){if w.workspace==id{let _=conn.map_window(w.id);let _=set_wm_state(&conn,w.id,wm_state,1);let _=set_atom_state(&conn,w.id,net_wm_state,net_wm_state_hidden,false);}else{let _=conn.unmap_window(w.id);let _=set_wm_state(&conn,w.id,wm_state,1);let _=set_atom_state(&conn,w.id,net_wm_state,net_wm_state_hidden,true);}}let _=conn.change_property32(xproto::PropMode::REPLACE,root,current,AtomEnum::CARDINAL,&[id-1]);arrange(&conn,root,&s);}else if key==24{let s=state.lock().unwrap();if let Some(w)=s.clients.values().find(|w|w.focused){if let Err(e)=request_close(&conn,w.id,wm_protocols,wm_delete){warn!(error=%e,"polite close failed")}}}}, Some(Event::ClientMessage(e))=>{
       let data=e.data.as_data32();
       if e.type_==net_close { let target=e.window; if target!=0 && target!=root { if let Err(err)=request_close(&conn,target,wm_protocols,wm_delete){warn!(window=target,error=%err,"EWMH close request failed");} } }
       else if e.type_==net_active { let target=e.window; let mut s=state.lock().unwrap(); if s.clients.get(&target).is_some_and(|w|w.workspace==s.active){for w in s.clients.values_mut(){w.focused=w.id==target;}let _=conn.map_window(target);if let Err(error)=focus_client(&conn,target,wm_protocols,wm_take_focus){warn!(window=target,%error,"ICCCM focus negotiation failed");}let _=conn.change_property32(xproto::PropMode::REPLACE,root,net_active_window,AtomEnum::WINDOW,&[target]);} }
       else if e.type_==net_wm_state {
         // EWMH action: 0=remove, 1=add, 2=toggle. Only fullscreen is
         // advertised until maximize/above/below have complete semantics.
         let action=data[0]; let first=data[1]; let second=data[2];
         for atom in [first,second].into_iter().filter(|a| *a!=0) {
           if atom==net_wm_state_fullscreen {
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
                 arrange(&conn,root,&s);
               }
             }
           }
         }
       }
       else if e.type_==desktop { let target=e.window; let requested=data[0]; let mut s=state.lock().unwrap(); if requested!=u32::MAX && requested<s.settings.workspace.count as u32 {let ws=requested+1;if let Some(w)=s.clients.get_mut(&target){w.workspace=ws;let _=conn.change_property32(xproto::PropMode::REPLACE,target,desktop,AtomEnum::CARDINAL,&[requested]);if ws==s.active{let _=conn.map_window(target);let _=set_wm_state(&conn,target,wm_state,1);let _=set_atom_state(&conn,target,net_wm_state,net_wm_state_hidden,false);}else{let _=conn.unmap_window(target);let _=set_wm_state(&conn,target,wm_state,1);let _=set_atom_state(&conn,target,net_wm_state,net_wm_state_hidden,true);}arrange(&conn,root,&s);}} }
       else if e.type_==current { let id=data[0].saturating_add(1); if id>=1 && id<=state.lock().unwrap().settings.workspace.count as u32 { let mut s=state.lock().unwrap();s.active=id;for w in s.clients.values_mut(){w.focused=false;}let _=conn.delete_property(root,net_active_window);for w in s.clients.values(){if w.workspace==id{let _=conn.map_window(w.id);let _=set_wm_state(&conn,w.id,wm_state,1);let _=set_atom_state(&conn,w.id,net_wm_state,net_wm_state_hidden,false);}else{let _=conn.unmap_window(w.id);let _=set_wm_state(&conn,w.id,wm_state,1);let _=set_atom_state(&conn,w.id,net_wm_state,net_wm_state_hidden,true);}}let _=conn.change_property32(xproto::PropMode::REPLACE,root,current,AtomEnum::CARDINAL,&[id-1]);arrange(&conn,root,&s); } }
     }, Some(_) =>{}, None=>thread::sleep(Duration::from_millis(8)) }
   // Some X servers (including Xvfb) expose RandR 1.5 monitor-object changes
   // without delivering the expected notification to an already-running WM.
   // Reconcile at a low frequency as a safety net; only arrange when geometry
   // or monitor identity actually changes.
   if last_monitor_refresh.elapsed() >= Duration::from_millis(250) {
       let mut s = state.lock().unwrap();
       refresh_monitor_state(&conn, root, &mut s, screen.width_in_pixels, screen.height_in_pixels);
       last_monitor_refresh = std::time::Instant::now();
   }
   conn.flush()?; }
}
fn refresh_monitor_state(conn: &RustConnection, root: Window, state: &mut State, width: u16, height: u16) {
    let discovered = discover_monitors(conn, root, width, height);
    let changed = discovered.len() != state.monitors.len()
        || discovered.iter().zip(&state.monitors).any(|(a, b)| {
            a.name != b.name || a.x != b.x || a.y != b.y
                || a.width != b.width || a.height != b.height || a.focused != b.focused
        });
    if !changed { return; }
    state.monitors = discovered;
    let fallback = state.monitors.iter().find(|m| m.focused)
        .or_else(|| state.monitors.first()).cloned().unwrap_or_default();
    let current_monitors = state.monitors.clone();
    for window in state.clients.values_mut() {
        if !current_monitors.iter().any(|m| m.name == window.monitor) {
            window.monitor = fallback.name.clone();
            if window.floating {
                let max_x = fallback.x + fallback.width.saturating_sub(window.width) as i32;
                let max_y = fallback.y + fallback.height.saturating_sub(window.height) as i32;
                window.x = window.x.clamp(fallback.x, max_x.max(fallback.x));
                window.y = window.y.clamp(fallback.y, max_y.max(fallback.y));
                let _ = conn.configure_window(window.id, &ConfigureWindowAux::new().x(window.x).y(window.y));
            }
        }
    }
    info!(count = state.monitors.len(), names = ?state.monitors.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), "RandR monitor topology refreshed");
    arrange(conn, root, state);
}
fn arrange(conn: &RustConnection, _root: Window, s: &State) {
    use mrx_core::layout::{arrange as calculate, LayoutKind, Rect};
    let layout = LayoutKind::parse(&s.settings.window.layout).unwrap_or(LayoutKind::MasterStack);
    let fallback = s.monitors.iter().find(|m| m.focused).or_else(|| s.monitors.first()).cloned().unwrap_or_default();
    let monitors = if s.monitors.is_empty() { vec![fallback] } else { s.monitors.clone() };
    for monitor in &monitors {
        let visible: Vec<&WindowInfo> = s.clients.values()
            .filter(|w| w.workspace == s.active && !w.floating && !w.fullscreen
                && (w.monitor == monitor.name || (w.monitor.is_empty() && monitor.focused)))
            .collect();
        if visible.is_empty() { continue; }
        let rects = calculate(
            Rect { x: monitor.x, y: monitor.y, width: monitor.width, height: monitor.height },
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
            MonitorInfo { name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080, focused: true },
            MonitorInfo { name: "HDMI-1".into(), x: 1920, y: 0, width: 2560, height: 1440, focused: false },
            MonitorInfo { name: "DP-2".into(), x: -1280, y: 0, width: 1280, height: 1024, focused: false },
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
    fn empty_monitor_registry_is_safe() {
        assert_eq!(monitor_for_geometry(&[], 0, 0, 800, 600), "");
    }
}
