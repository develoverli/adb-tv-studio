//! Reconnects saved TVs while the app is open: shortly after launch, then every
//! [`INTERVAL`]. Attempts are silent; a TV the user disconnected by hand is skipped until
//! they reconnect it.

use super::commands::{connect, mdns_services};
use super::parse::{DeviceStatus, MdnsService};
use super::saved::{self, SavedTv};
use super::{AdbState, run};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use tauri::{AppHandle, Manager};

/// Gap between rounds.
const INTERVAL: Duration = Duration::from_secs(30);
/// First round after launch, once `track-devices` has reported what is already connected.
const FIRST_DELAY: Duration = Duration::from_secs(3);
/// Port check before `adb connect`, whose own timeout on an unreachable TV is the OS one (~20 s).
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Spawns the reconnect loop. It stops when the app shuts down.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        sleep(FIRST_DELAY).await;
        while !app.state::<AdbState>().is_shutting_down() {
            reconnect_saved(&app).await;
            sleep(INTERVAL).await;
        }
    });
}

/// No async timer without a direct tokio dependency; a blocking sleep on the blocking pool is fine.
async fn sleep(duration: Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(duration)).await;
}

/// One round over the saved TVs that want auto-reconnect and are not connected or waiting
/// for authorization.
async fn reconnect_saved(app: &AppHandle) {
    let state = app.state::<AdbState>();
    let entries = state.entries().clone();
    let targets: Vec<(SavedTv, Option<DeviceStatus>)> = state
        .saved()
        .list()
        .iter()
        .filter(|tv| tv.auto_reconnect)
        .filter_map(|tv| {
            let status = entries
                .iter()
                .find(|(serial, _)| *serial == tv.address)
                .map(|(_, status)| *status);
            matches!(status, None | Some(DeviceStatus::Offline)).then(|| (tv.clone(), status))
        })
        .collect();

    // Read once per round, and only if some TV no longer answers at its saved address.
    let mut services: Option<Vec<MdnsService>> = None;
    for (tv, status) in targets {
        reconnect_one(app, &tv, status, &mut services).await;
    }
}

async fn reconnect_one(
    app: &AppHandle,
    tv: &SavedTv,
    status: Option<DeviceStatus>,
    services: &mut Option<Vec<MdnsService>>,
) {
    let address = if is_reachable(&tv.address).await {
        tv.address.clone()
    } else {
        if services.is_none() {
            *services = Some(mdns_services(app).await.unwrap_or_default());
        }
        let found = services
            .as_deref()
            .and_then(|list| saved::moved_address(tv, list));
        match found {
            Some(address) => address.to_owned(),
            None => return,
        }
    };

    // A stuck "offline" transport, or the old address of a TV that moved, must go first or
    // `adb connect` reports "already connected" and the TV stays offline (or listed twice).
    if status.is_some()
        && let Err(error) = run(app, &["disconnect", &tv.address]).await
    {
        eprintln!("disconnect before auto-reconnect failed: {error}");
    }
    // Once connected, the tracker saves the new address and info.
    if let Err(error) = connect(app, &address).await {
        eprintln!("auto-reconnect to {address} failed: {error}");
    }
}

/// True when something accepts TCP connections at `address` within [`PROBE_TIMEOUT`].
async fn is_reachable(address: &str) -> bool {
    let address = address.to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        address
            .to_socket_addrs()
            .ok()
            .and_then(|mut resolved| resolved.next())
            .is_some_and(|socket| TcpStream::connect_timeout(&socket, PROBE_TIMEOUT).is_ok())
    })
    .await
    .unwrap_or(false)
}
