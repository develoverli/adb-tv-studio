//! Tauri commands exposed to the webview. Input is validated here before it reaches adb.

use super::parse::{self, ConnectStatus, MdnsKind, MdnsService};
use super::scan;
use super::{AdbError, AdbState, Device, run, tracker};
use serde::Serialize;
use tauri::{AppHandle, State};

/// How a scan result was found, which decides what the UI offers for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscoveryKind {
    /// Android 11+ wireless debugging, announced by mDNS. Connect works once paired.
    Wireless,
    /// A TV with its "Pair device with code" dialog open. Needs the code.
    Pairing,
    /// Port 5555 open: legacy network debugging. Connect directly.
    Network,
}

/// One TV found by `adb_scan`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovered {
    pub address: String,
    /// mDNS instance name, when announced.
    pub name: Option<String>,
    pub kind: DiscoveryKind,
}

/// The connection `adb_pair` made after pairing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    /// Connect address (`ip:port`) found by mDNS, which differs from the pairing port.
    pub address: String,
    pub status: ConnectStatus,
}

/// Result of a successful `adb_pair`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairResult {
    /// Set when mDNS revealed the connect port; `None` means the user must connect with the
    /// IP and port shown under Wireless debugging.
    pub connection: Option<Connection>,
}

/// Current device list, for the first render before any `devices-changed` event.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects State by value"
)]
#[tauri::command]
pub fn adb_list(state: State<'_, AdbState>) -> Vec<Device> {
    state.devices().clone()
}

/// Looks for TVs by mDNS and by probing port 5555 on the local subnets, in parallel.
#[tauri::command]
pub async fn adb_scan(app: AppHandle) -> Result<Vec<Discovered>, AdbError> {
    let probe = tauri::async_runtime::spawn_blocking(|| {
        let hosts = scan::hosts_to_probe(&scan::local_networks());
        scan::probe(&hosts)
    });
    // mDNS can be unavailable (disabled backend, firewall); the port scan still gives results.
    let mdns = mdns_services(&app).await.unwrap_or_default();
    let open_hosts = probe
        .await
        .map_err(|error| AdbError::Command(error.to_string()))?;

    let mut found: Vec<Discovered> = mdns
        .into_iter()
        .map(|service| Discovered {
            address: service.address,
            name: Some(service.instance),
            kind: match service.kind {
                MdnsKind::Connect => DiscoveryKind::Wireless,
                MdnsKind::Pairing => DiscoveryKind::Pairing,
            },
        })
        .collect();
    found.extend(open_hosts.into_iter().map(|host| Discovered {
        address: format!("{host}:{}", scan::LEGACY_PORT),
        name: None,
        kind: DiscoveryKind::Network,
    }));
    Ok(found)
}

/// Connects to `host[:port]` (port 5555 when omitted).
#[tauri::command]
pub async fn adb_connect(app: AppHandle, address: String) -> Result<ConnectStatus, AdbError> {
    let address = parse::address(&address, Some(scan::LEGACY_PORT))?;
    connect(&app, &address).await
}

/// Pairs with the code from the TV's dialog, then connects if mDNS reveals the connect port.
#[tauri::command]
pub async fn adb_pair(
    app: AppHandle,
    address: String,
    code: String,
) -> Result<PairResult, AdbError> {
    let address = parse::address(&address, None)?;
    let code = parse::pairing_code(&code)?;
    let output = run(&app, &["pair", &address, &code]).await?;
    parse::pair_outcome(&output.text())?;

    let host = parse::host_of(&address);
    let connect_address = mdns_services(&app)
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|service| {
            service.kind == MdnsKind::Connect && parse::host_of(&service.address) == host
        })
        .map(|service| service.address);
    let connection = match connect_address {
        Some(address) => {
            let status = connect(&app, &address).await?;
            Some(Connection { address, status })
        }
        None => None,
    };
    Ok(PairResult { connection })
}

/// Disconnects a TV from the current list. A saved TV stays listed but is no longer
/// reconnected automatically until the user reconnects it.
#[tauri::command]
pub async fn adb_disconnect(
    app: AppHandle,
    state: State<'_, AdbState>,
    serial: String,
) -> Result<(), AdbError> {
    state.require_device(&serial)?;
    set_auto_reconnect(&state, &serial, false);
    let output = run(&app, &["disconnect", &serial]).await?;
    parse::disconnect_outcome(&output.text())
}

/// Removes a saved TV and disconnects it.
#[tauri::command]
pub async fn adb_forget(
    app: AppHandle,
    state: State<'_, AdbState>,
    serial: String,
) -> Result<(), AdbError> {
    {
        let mut saved = state.saved();
        if !saved.forget(&serial) {
            return Err(AdbError::UnknownDevice(serial));
        }
        saved.save_or_report();
    }
    tracker::emit_devices(&app);
    // It may not be connected at all; forgetting it already succeeded.
    if let Err(error) = run(&app, &["disconnect", &serial]).await {
        eprintln!("disconnect after forget failed: {error}");
    }
    Ok(())
}

/// Drops a stuck (offline) connection and connects again to the same address.
#[tauri::command]
pub async fn adb_reconnect(
    app: AppHandle,
    state: State<'_, AdbState>,
    serial: String,
) -> Result<ConnectStatus, AdbError> {
    state.require_device(&serial)?;
    set_auto_reconnect(&state, &serial, true);
    // Only `ip:port` serials can be reconnected by address; mDNS names cannot.
    let address = parse::address(&serial, None)?;
    // The old transport may already be gone; a failed disconnect must not block the new connect.
    if let Err(error) = run(&app, &["disconnect", &address]).await {
        eprintln!("disconnect before reconnect failed: {error}");
    }
    connect(&app, &address).await
}

/// Turns auto-reconnect on or off for a saved TV and saves it; nothing happens for other TVs.
fn set_auto_reconnect(state: &AdbState, serial: &str, enabled: bool) {
    let mut saved = state.saved();
    if saved.set_auto_reconnect(serial, enabled) {
        saved.save_or_report();
    }
}

pub(super) async fn connect(app: &AppHandle, address: &str) -> Result<ConnectStatus, AdbError> {
    let output = run(app, &["connect", address]).await?;
    parse::connect_outcome(&output.text())
}

pub(super) async fn mdns_services(app: &AppHandle) -> Result<Vec<MdnsService>, AdbError> {
    let output = run(app, &["mdns", "services"]).await?;
    if output.success {
        Ok(parse::mdns_services(&output.stdout))
    } else {
        Err(AdbError::Command(output.text().trim().to_owned()))
    }
}
