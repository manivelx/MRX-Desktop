//! Shared, versioned data model for MRX Desktop.
//!
//! This crate deliberately contains no X11 or Qt code so the protocol and
//! settings model can be tested without starting a graphical session.

pub mod layout;

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub const IPC_PROTOCOL_VERSION: u32 = 3;
pub const SOCKET_NAME: &str = "mrx.sock";
pub const DEFAULT_WORKSPACES: usize = 9;
pub const MAX_WORKSPACES: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Settings {
    pub animation: AnimationSettings,
    pub appearance: AppearanceSettings,
    pub workspace: WorkspaceSettings,
    pub window: WindowSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AnimationSettings {
    pub enabled: bool,
    pub workspace_duration_ms: u64,
    pub window_open: bool,
    pub window_close: bool,
    pub easing: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: String,
    pub accent: String,
    pub corner_radius: u32,
    pub panel_opacity: f32,
    pub gaps: u32,
    pub border_width: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WorkspaceSettings {
    pub count: usize,
    pub wrap: bool,
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WindowSettings {
    pub layout: String,
    pub focus_follows_pointer: bool,
    pub master_ratio: f32,
    pub floating_modifier: String,
}

impl Default for AnimationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            workspace_duration_ms: 280,
            window_open: true,
            window_close: true,
            easing: "ease-out-cubic".into(),
        }
    }
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: "amber-gold".into(),
            accent: "#D6A84B".into(),
            corner_radius: 14,
            panel_opacity: 0.94,
            gaps: 8,
            border_width: 2,
        }
    }
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            count: DEFAULT_WORKSPACES,
            wrap: true,
            names: (1..=DEFAULT_WORKSPACES).map(|n| n.to_string()).collect(),
        }
    }
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            layout: "master-stack".into(),
            focus_follows_pointer: false,
            master_ratio: 0.58,
            floating_modifier: "super".into(),
        }
    }
}

impl Settings {
    /// Resolve the per-session configuration directory. The explicit MRX
    /// override keeps the session isolated from other desktop environments.
    pub fn config_dir() -> PathBuf {
        if let Some(path) = std::env::var_os("MRX_CONFIG_HOME") {
            return PathBuf::from(path);
        }
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
            })
            .join("mrx")
    }

    pub fn path() -> PathBuf {
        Self::config_dir().join("settings.toml")
    }

    pub fn validate_profile_name(name:&str) -> anyhow::Result<()> {
        anyhow::ensure!(!name.is_empty() && name.len()<=48,"profile name must contain 1-48 characters");
        anyhow::ensure!(name.bytes().all(|byte|byte.is_ascii_alphanumeric() || byte==b'-' || byte==b'_'),
            "profile name may contain only ASCII letters, digits, '-' and '_'");
        Ok(())
    }

    fn profiles_dir() -> anyhow::Result<PathBuf> {
        let dir=Self::config_dir().join("profiles");
        match fs::symlink_metadata(&dir) {
            Ok(metadata) if metadata.file_type().is_symlink()=>anyhow::bail!("refusing to use a symlink as the profiles directory"),
            Ok(metadata) if !metadata.is_dir()=>anyhow::bail!("profiles path exists but is not a directory"),
            Ok(_)=>{},
            Err(error) if error.kind()==std::io::ErrorKind::NotFound=>fs::create_dir_all(&dir)?,
            Err(error)=>return Err(error.into()),
        }
        Ok(dir)
    }

    pub fn profile_path(name:&str) -> anyhow::Result<PathBuf> {
        Self::validate_profile_name(name)?;
        Ok(Self::config_dir().join("profiles").join(format!("{name}.toml")))
    }

    pub fn load_profile(name:&str) -> anyhow::Result<Self> {
        let dir=Self::profiles_dir()?;
        let path=Self::profile_path(name)?;
        let metadata=fs::symlink_metadata(&path)?;
        anyhow::ensure!(!metadata.file_type().is_symlink() && metadata.is_file(),"profile is not a regular file");
        let contents=fs::read_to_string(path)?;
        let settings:Self=toml::from_str(&contents)?;
        settings.validate()?;
        let _=dir;
        Ok(settings)
    }

    pub fn save_profile_atomic(&self,name:&str) -> anyhow::Result<()> {
        self.validate()?;
        let _=Self::profiles_dir()?;
        let path=Self::profile_path(name)?;
        self.save_to_path_atomic(&path)
    }

    pub fn load() -> anyhow::Result<Self> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(&path)?;
        let settings: Self = toml::from_str(&contents)?;
        settings.validate()?;
        Ok(settings)
    }

    /// Reject invalid values before they reach the window manager or shell.
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            (1..=MAX_WORKSPACES).contains(&self.workspace.count),
            "workspace count must be between 1 and {MAX_WORKSPACES}"
        );
        anyhow::ensure!(
            self.workspace.names.len() <= MAX_WORKSPACES,
            "workspace names cannot exceed {MAX_WORKSPACES} entries"
        );
        anyhow::ensure!(
            (0.2..=1.0).contains(&self.appearance.panel_opacity),
            "panel opacity must be between 0.2 and 1.0"
        );
        anyhow::ensure!(
            (0.2..=0.8).contains(&self.window.master_ratio),
            "master ratio must be between 0.2 and 0.8"
        );
        anyhow::ensure!(
            self.animation.workspace_duration_ms <= 2_000,
            "workspace animation duration cannot exceed 2000 ms"
        );
        anyhow::ensure!(
            matches!(self.window.layout.as_str(), "master-stack" | "monocle" | "columns"),
            "unsupported layout: {}",
            self.window.layout
        );
        anyhow::ensure!(
            self.appearance.accent.len() == 7
                && self.appearance.accent.starts_with('#')
                && self.appearance.accent[1..].bytes().all(|b| b.is_ascii_hexdigit()),
            "accent must be a six-digit hexadecimal color"
        );
        Ok(())
    }

    /// Persist using a same-directory temporary file, fsync and atomic rename.
    pub fn save_atomic(&self) -> anyhow::Result<()> {
        self.validate()?;
        self.save_to_path_atomic(&Self::path())
    }

    fn save_to_path_atomic(&self,path:&Path) -> anyhow::Result<()> {
        self.validate()?;
        let parent=path.parent().ok_or_else(||anyhow::anyhow!("settings path has no parent directory"))?;
        fs::create_dir_all(parent)?;
        let filename=path.file_name().and_then(|value|value.to_str()).ok_or_else(||anyhow::anyhow!("settings filename is invalid"))?;
        let temp=parent.join(format!(".{filename}.{}.tmp",std::process::id()));
        let serialized=toml::to_string_pretty(self)?;
        let mut file=OpenOptions::new().write(true).create_new(true).open(&temp).or_else(|error| {
            if error.kind()==std::io::ErrorKind::AlreadyExists {
                fs::remove_file(&temp)?;
                OpenOptions::new().write(true).create_new(true).open(&temp)
            } else { Err(error) }
        })?;
        let result=(||->anyhow::Result<()> {
            file.write_all(serialized.as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp,path)?;
            if let Ok(directory)=OpenOptions::new().read(true).open(parent){let _=directory.sync_all();}
            Ok(())
        })();
        if result.is_err(){let _=fs::remove_file(&temp);}
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    pub class: String,
    pub workspace: u32,
    #[serde(default)]
    pub monitor: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub focused: bool,
    pub floating: bool,
    pub fullscreen: bool,
    #[serde(default)]
    pub maximized_horz: bool,
    #[serde(default)]
    pub maximized_vert: bool,
    #[serde(default)]
    pub minimized: bool,
    #[serde(default)]
    pub above: bool,
    #[serde(default)]
    pub below: bool,
    /// ICCCM WM_TRANSIENT_FOR parent XID, or zero for an ordinary toplevel.
    #[serde(default)]
    pub transient_for: u32,
    /// EWMH modal-dialog state; modal descendants take focus over their parent.
    #[serde(default)]
    pub modal: bool,
    /// ICCCM WM_HINTS urgency bit; the client owns setting and clearing it.
    #[serde(default)]
    pub urgent: bool,
    /// EWMH dock/panel clients are sticky and excluded from workspace tiling.
    #[serde(default)]
    pub dock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceInfo {
    pub id: u32,
    /// Monitor that owns this workspace instance. Workspace numbers are
    /// local to a monitor and may therefore repeat across outputs.
    #[serde(default)]
    pub monitor: String,
    pub name: String,
    pub active: bool,
    pub windows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub protocol_version: u32,
    pub compositor: String,
    pub active_workspace: u32,
    pub workspaces: Vec<WorkspaceInfo>,
    pub windows: Vec<WindowInfo>,
    pub settings: Settings,
    pub monitors: Vec<MonitorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct MonitorInfo {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub focused: bool,
    /// Active workspace local to this monitor. Defaults to workspace 1 for
    /// snapshots produced by earlier protocol-v2 MRX builds.
    #[serde(default = "default_monitor_workspace")]
    pub active_workspace: u32,
}

fn default_monitor_workspace() -> u32 { 1 }


/// Versioned IPC request envelope. IDs correlate each response with its
/// originating request, even when clients pipeline or retry operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RequestEnvelope {
    pub protocol_version: u32,
    pub request_id: u64,
    pub request: Request,
}

impl RequestEnvelope {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.protocol_version == IPC_PROTOCOL_VERSION,
            "unsupported IPC protocol version {}; expected {}",
            self.protocol_version,
            IPC_PROTOCOL_VERSION
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Request {
    Snapshot,
    Subscribe,
    SwitchWorkspace { id: u32 },
    SwitchWorkspaceOnMonitor { monitor: String, id: u32 },
    SwitchRelative { direction: String },
    FocusWindow { id: u32 },
    CloseWindow { id: u32 },
    MoveWindow { id: u32, workspace: u32 },
    SetSetting { key: String, value: serde_json::Value },
    ReloadSettings,
    Ping,
    GetOperation { id: u64 },
    UndoSettings,
    ResetSettings,
    ExportSettingsProfile { name: String },
    ImportSettingsProfile { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub protocol_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<u64>,
    pub ok: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<Snapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<OperationResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<OperationError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Queued,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationResult {
    pub id: u64,
    pub kind: String,
    pub status: OperationStatus,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<OperationError>,
}

impl Response {
    pub fn ok(message: impl Into<String>) -> Self {
        Self { protocol_version: IPC_PROTOCOL_VERSION, request_id: None, ok: true, message: message.into(), snapshot: None, settings: None, data: None, operation: None, error: None }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self { protocol_version: IPC_PROTOCOL_VERSION, request_id: None, ok: false, message: message.into(), snapshot: None, settings: None, data: None, operation: None, error: None }
    }

    pub fn err_code(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        let error=OperationError{code:code.into(),message:message.into(),retryable};
        let mut response=Self::err(error.message.clone());
        response.error=Some(error);
        response
    }

    pub fn with_operation(operation: OperationResult) -> Self {
        let mut response=Self::ok(operation.message.clone());
        response.operation=Some(operation);
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_request_envelope_round_trips_and_validates_version() {
        let envelope = RequestEnvelope {
            protocol_version: IPC_PROTOCOL_VERSION,
            request_id: 42,
            request: Request::Ping,
        };
        envelope.validate().unwrap();
        let encoded = serde_json::to_string(&envelope).unwrap();
        let decoded: RequestEnvelope = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn ipc_request_envelope_rejects_unknown_protocol_versions() {
        let envelope = RequestEnvelope {
            protocol_version: IPC_PROTOCOL_VERSION + 1,
            request_id: 99,
            request: Request::Ping,
        };
        assert!(envelope.validate().is_err());
    }

    #[test]
    fn operation_result_round_trips_structured_status_and_error() {
        let operation=OperationResult{
            id:17,
            kind:"move_window".into(),
            status:OperationStatus::Failed,
            message:"window 999 is not managed".into(),
            error:Some(OperationError{code:"window_not_found".into(),message:"window 999 is not managed".into(),retryable:false}),
        };
        let encoded=serde_json::to_string(&operation).unwrap();
        assert!(encoded.contains("\"status\":\"failed\""));
        let decoded:OperationResult=serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded,operation);
    }

    #[test]
    fn get_operation_request_round_trips() {
        let request=Request::GetOperation{id:123};
        let encoded=serde_json::to_string(&request).unwrap();
        let decoded:Request=serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded,request);
    }

    #[test]
    fn response_includes_optional_operation_and_error_details() {
        let operation=OperationResult{id:1,kind:"reload_settings".into(),status:OperationStatus::Queued,message:"operation queued".into(),error:None};
        let response=Response::with_operation(operation.clone());
        assert!(response.ok);
        assert_eq!(response.operation,Some(operation));
        let error=Response::err_code("invalid_workspace","workspace out of range",false);
        assert_eq!(error.error.unwrap().code,"invalid_workspace");
    }

    #[test]
    fn settings_profile_names_reject_path_traversal_and_unsafe_characters() {
        assert!(Settings::validate_profile_name("amber-gold").is_ok());
        assert!(Settings::validate_profile_name("work_2026").is_ok());
        assert!(Settings::validate_profile_name("../settings").is_err());
        assert!(Settings::validate_profile_name("name/child").is_err());
        assert!(Settings::validate_profile_name("").is_err());
        assert!(Settings::validate_profile_name(&"x".repeat(49)).is_err());
    }

    #[test]
    fn settings_profile_requests_round_trip() {
        for request in [
            Request::UndoSettings,
            Request::ResetSettings,
            Request::ExportSettingsProfile{name:"amber-gold".into()},
            Request::ImportSettingsProfile{name:"work_2026".into()},
        ] {
            let encoded=serde_json::to_string(&request).unwrap();
            let decoded:Request=serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded,request);
        }
    }

    #[test]
    fn old_workspace_snapshots_default_missing_monitor_fields() {
        let workspace: WorkspaceInfo = serde_json::from_str(r#"{"id":1,"name":"1","active":true,"windows":0}"#).unwrap();
        assert_eq!(workspace.monitor, "");
    }

    #[test]
    fn old_window_snapshots_default_missing_monitor_fields() {
        let window: WindowInfo = serde_json::from_str(r#"{"id":42,"workspace":1}"#).unwrap();
        assert_eq!(window.id, 42);
        assert_eq!(window.monitor, "");
        assert!(!window.urgent);
    }

    #[test]
    fn monitor_workspace_defaults_for_older_snapshots() {
        let monitor: MonitorInfo = serde_json::from_str(
            r#"{"name":"DP-1","x":0,"y":0,"width":1920,"height":1080,"focused":true}"#
        ).unwrap();
        assert_eq!(monitor.active_workspace, 1);
    }

    #[test]
    fn subscribe_request_round_trips() {
        let envelope = RequestEnvelope {
            protocol_version: IPC_PROTOCOL_VERSION,
            request_id: 77,
            request: Request::Subscribe,
        };
        envelope.validate().unwrap();
        let encoded = serde_json::to_string(&envelope).unwrap();
        let decoded: RequestEnvelope = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn monitor_workspace_switch_request_round_trips() {
        let request = Request::SwitchWorkspaceOnMonitor {
            monitor: "HDMI-1".into(),
            id: 3,
        };
        let encoded = serde_json::to_string(&request).unwrap();
        let decoded: Request = serde_json::from_str(&encoded).unwrap();
        assert!(matches!(decoded, Request::SwitchWorkspaceOnMonitor { monitor, id: 3 } if monitor == "HDMI-1"));
    }

    #[test]
    fn defaults_are_safe_for_kepler_baseline() {
        let settings = Settings::default();
        assert_eq!(settings.workspace.count, 9);
        assert_eq!(settings.appearance.theme, "amber-gold");
        assert_eq!(settings.animation.workspace_duration_ms, 280);
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn settings_round_trip_toml() {
        let settings = Settings::default();
        let encoded = toml::to_string(&settings).unwrap();
        let decoded: Settings = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded, settings);
    }

    #[test]
    fn old_partial_settings_receive_defaults() {
        let decoded: Settings = toml::from_str("[workspace]\ncount = 5\n").unwrap();
        assert_eq!(decoded.workspace.count, 5);
        assert_eq!(decoded.appearance.theme, "amber-gold");
        assert!(decoded.validate().is_ok());
    }

    #[test]
    fn invalid_values_are_rejected() {
        let mut settings = Settings::default();
        settings.workspace.count = 0;
        assert!(settings.validate().is_err());
        settings = Settings::default();
        settings.appearance.accent = "not-a-color".into();
        assert!(settings.validate().is_err());
    }
}
