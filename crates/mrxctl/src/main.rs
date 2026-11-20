use anyhow::{Context,Result};
use mrx_core::{Request,Response};
use std::{io::{BufRead,BufReader,Read,Write},os::unix::net::UnixStream,path::PathBuf,time::Duration};
fn socket()->PathBuf{std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(||PathBuf::from(format!("/tmp/mrx-{}",std::env::var("USER").unwrap_or_else(|_|"user".into())))).join("mrx.sock")}
fn call(req:Request)->Result<Response>{
    let mut stream=UnixStream::connect(socket()).context("MRX WM is not running in this session")?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    writeln!(stream,"{}",serde_json::to_string(&req)?)?;
    let mut line=String::new();
    let bytes=BufReader::new(stream).take(65_537).read_line(&mut line)?;
    anyhow::ensure!(bytes>0 && bytes<=65_536 && line.ends_with('\\n'),"MRX WM returned an empty, oversized, or unterminated IPC response");
    Ok(serde_json::from_str(&line).context("invalid MRX WM IPC response")?)
}
fn main()->Result<()>{let mut a=std::env::args().skip(1);let cmd=a.next().unwrap_or_else(||"help".into());let req=match cmd.as_str(){"snapshot"=>Request::Snapshot,"ping"=>Request::Ping,"workspace"=>{let v=a.next().unwrap_or_else(||"1".into());if v=="next"||v=="previous"{Request::SwitchRelative{direction:if v=="next"{"next"}else{"previous"}.into()}}else{Request::SwitchWorkspace{id:v.parse()?}}},"focus"=>Request::FocusWindow{id:a.next().context("window XID required")?.parse()?},"close"=>Request::CloseWindow{id:a.next().context("window XID required")?.parse()?},"move"=>Request::MoveWindow{id:a.next().context("window XID required")?.parse()?,workspace:a.next().context("workspace required")?.parse()?},"set"=>Request::SetSetting{key:a.next().context("setting key required")?,value:serde_json::from_str(&a.next().context("JSON value required")?)?},"reload"=>Request::ReloadSettings,_=>{println!("mrxctl snapshot|ping|workspace <1-9|next|previous>|focus <xid>|close <xid>|move <xid> <workspace>|set <key> <json-value>|reload");return Ok(())}};let r=call(req)?;if let Some(s)=r.snapshot{println!("{}",serde_json::to_string(&s)?)}else{println!("{}",serde_json::to_string(&r)?)}if !r.ok{std::process::exit(1)}Ok(())}
