//! TVs remembered across launches, kept in `devices.json` in the app config directory.
//!
//! A network TV is saved the first time it connects and is authorized; it stays in the list
//! while offline and is reconnected by [`super::reconnect`] until the user forgets it.

use super::parse::{DeviceInfo, MdnsKind, MdnsService, host_of};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// File name inside the app config directory.
pub const FILE_NAME: &str = "devices.json";
/// Bumped when the file layout changes incompatibly.
const FILE_VERSION: u32 = 1;

/// One saved TV.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedTv {
    /// Last connect address (`ip:port`); also the device id while it is offline.
    pub address: String,
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub android_version: Option<String>,
    /// `ro.serialno`. Identifies the TV when its wireless debugging port (or IP) changes.
    pub serial_number: Option<String>,
    /// Unix seconds of the last connection.
    pub last_connected: u64,
    /// Turned off by a manual disconnect, back on with Reconnect or a new connection.
    pub auto_reconnect: bool,
    /// Stock launchers this app disabled, so "Restore original launcher" can enable them again.
    #[serde(default)]
    pub disabled_launchers: Vec<String>,
}

impl SavedTv {
    /// Last info read from the TV, to show it while offline.
    pub fn info(&self) -> DeviceInfo {
        DeviceInfo {
            name: self.name.clone(),
            manufacturer: self.manufacturer.clone(),
            model: self.model.clone(),
            android_version: self.android_version.clone(),
            serial_number: self.serial_number.clone(),
        }
    }

    fn is_same_tv(&self, address: &str, serial_number: Option<&str>) -> bool {
        match (self.serial_number.as_deref(), serial_number) {
            (Some(saved), Some(current)) => saved == current,
            _ => self.address == address,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SavedFile {
    version: u32,
    tvs: Vec<SavedTv>,
}

/// The saved TVs and where they are written. Without a path (no config dir) nothing is written.
#[derive(Debug, Default)]
pub struct SavedTvs {
    path: Option<PathBuf>,
    tvs: Vec<SavedTv>,
}

impl SavedTvs {
    /// Reads the file. A missing file is an empty list; an unreadable one is reported and
    /// replaced on the next save rather than blocking the app.
    pub fn load(path: PathBuf) -> Self {
        let tvs = match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<SavedFile>(&text) {
                Ok(file) if file.version == FILE_VERSION => file.tvs,
                Ok(file) => {
                    eprintln!(
                        "ignoring {}: unknown version {}",
                        path.display(),
                        file.version
                    );
                    Vec::new()
                }
                Err(error) => {
                    eprintln!("ignoring {}: {error}", path.display());
                    Vec::new()
                }
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                eprintln!("could not read {}: {error}", path.display());
                Vec::new()
            }
        };
        Self {
            path: Some(path),
            tvs,
        }
    }

    pub fn list(&self) -> &[SavedTv] {
        &self.tvs
    }

    pub fn find(&self, address: &str) -> Option<&SavedTv> {
        self.tvs.iter().find(|tv| tv.address == address)
    }

    /// Records a TV that just connected: adds it, or updates the saved one (matched by serial
    /// number, else by address) with the new address and info, and turns auto-reconnect back on.
    pub fn remember(&mut self, address: &str, info: &DeviceInfo, now: u64) {
        let serial_number = info.serial_number.as_deref();
        let tv = SavedTv {
            address: address.to_owned(),
            name: info.name.clone(),
            manufacturer: info.manufacturer.clone(),
            model: info.model.clone(),
            android_version: info.android_version.clone(),
            serial_number: info.serial_number.clone(),
            last_connected: now,
            auto_reconnect: true,
            disabled_launchers: Vec::new(),
        };
        let by_serial = serial_number.and_then(|serial| {
            self.tvs
                .iter()
                .position(|saved| saved.serial_number.as_deref() == Some(serial))
        });
        let index =
            by_serial.or_else(|| self.tvs.iter().position(|saved| saved.address == address));
        let index = if let Some(index) = index {
            let disabled_launchers = std::mem::take(&mut self.tvs[index].disabled_launchers);
            self.tvs[index] = SavedTv {
                disabled_launchers,
                ..tv
            };
            index
        } else {
            self.tvs.push(tv);
            self.tvs.len() - 1
        };
        // Any other entry for this TV, or one still holding its new address, is stale.
        let mut position = 0;
        self.tvs.retain(|saved| {
            let current = position;
            position += 1;
            current == index
                || (!saved.is_same_tv(address, serial_number) && saved.address != address)
        });
    }

    /// Removes a TV. Returns false when it was not saved.
    pub fn forget(&mut self, address: &str) -> bool {
        let before = self.tvs.len();
        self.tvs.retain(|tv| tv.address != address);
        self.tvs.len() != before
    }

    /// Returns false when the TV is not saved or already had that value.
    pub fn set_auto_reconnect(&mut self, address: &str, enabled: bool) -> bool {
        match self.tvs.iter_mut().find(|tv| tv.address == address) {
            Some(tv) if tv.auto_reconnect != enabled => {
                tv.auto_reconnect = enabled;
                true
            }
            _ => false,
        }
    }

    /// Mutable access to one saved TV, e.g. to record disabled launchers. Call `save` after.
    pub fn find_mut(&mut self, address: &str) -> Option<&mut SavedTv> {
        self.tvs.iter_mut().find(|tv| tv.address == address)
    }

    /// [`Self::save`], reporting a failure instead of returning it: the list in memory stays
    /// correct for this session either way.
    pub fn save_or_report(&self) {
        if let Err(error) = self.save() {
            eprintln!("could not save {FILE_NAME}: {error}");
        }
    }

    /// Writes the file through a temporary one, so a crash mid-write never leaves it truncated.
    pub fn save(&self) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = SavedFile {
            version: FILE_VERSION,
            tvs: self.tvs.clone(),
        };
        let text = serde_json::to_string_pretty(&file).map_err(io::Error::other)?;
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, text)?;
        std::fs::rename(&temporary, path)
    }
}

/// Current time in Unix seconds (0 if the clock is before 1970).
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// Where a saved TV is announced now, when its old address no longer answers.
///
/// Wireless debugging picks a new port after each reboot. The mDNS instance is
/// `adb-<serialno>-<suffix>`, so the serial number finds the TV even if its IP changed too;
/// without one, the same IP on a different port is taken.
pub fn moved_address<'a>(tv: &SavedTv, services: &'a [MdnsService]) -> Option<&'a str> {
    let mut connect = services
        .iter()
        .filter(|service| service.kind == MdnsKind::Connect && service.address != tv.address);
    let found = match tv.serial_number.as_deref() {
        Some(serial) => {
            let exact = format!("adb-{serial}");
            let prefix = format!("{exact}-");
            connect
                .find(|service| service.instance == exact || service.instance.starts_with(&prefix))
        }
        None => connect.find(|service| host_of(&service.address) == host_of(&tv.address)),
    };
    found.map(|service| service.address.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(serial: Option<&str>, name: &str) -> DeviceInfo {
        DeviceInfo {
            name: Some(name.to_owned()),
            serial_number: serial.map(str::to_owned),
            ..DeviceInfo::default()
        }
    }

    fn service(instance: &str, address: &str) -> MdnsService {
        MdnsService {
            instance: instance.to_owned(),
            kind: MdnsKind::Connect,
            address: address.to_owned(),
        }
    }

    fn addresses(tvs: &SavedTvs) -> Vec<&str> {
        tvs.list().iter().map(|tv| tv.address.as_str()).collect()
    }

    #[test]
    fn remember_should_add_a_new_tv() {
        let mut tvs = SavedTvs::default();
        tvs.remember("192.168.1.50:5555", &info(None, "Living room"), 10);
        assert_eq!(addresses(&tvs), vec!["192.168.1.50:5555"]);
    }

    #[test]
    fn remember_should_update_same_address() {
        let mut tvs = SavedTvs::default();
        tvs.remember("192.168.1.50:5555", &info(None, "Old"), 10);
        tvs.remember("192.168.1.50:5555", &info(None, "New"), 20);
        assert_eq!(tvs.list()[0].name.as_deref(), Some("New"));
    }

    #[test]
    fn remember_should_move_tv_matched_by_serial_number() {
        let mut tvs = SavedTvs::default();
        tvs.remember("192.168.1.50:40001", &info(Some("ABC"), "TV"), 10);
        tvs.remember("192.168.1.50:40002", &info(Some("ABC"), "TV"), 20);
        assert_eq!(addresses(&tvs), vec!["192.168.1.50:40002"]);
    }

    #[test]
    fn remember_should_keep_one_entry_per_address() {
        let mut tvs = SavedTvs::default();
        tvs.remember("192.168.1.50:5555", &info(None, "Old"), 10);
        tvs.remember("192.168.1.60:5555", &info(Some("ABC"), "TV"), 10);
        tvs.remember("192.168.1.50:5555", &info(Some("ABC"), "TV"), 20);
        assert_eq!(addresses(&tvs), vec!["192.168.1.50:5555"]);
    }

    #[test]
    fn remember_should_turn_auto_reconnect_back_on() {
        let mut tvs = SavedTvs::default();
        tvs.remember("192.168.1.50:5555", &info(None, "TV"), 10);
        tvs.set_auto_reconnect("192.168.1.50:5555", false);
        tvs.remember("192.168.1.50:5555", &info(None, "TV"), 20);
        assert!(tvs.list()[0].auto_reconnect);
    }

    #[test]
    fn remember_should_keep_disabled_launchers() {
        let mut tvs = SavedTvs::default();
        tvs.remember("192.168.1.50:5555", &info(None, "TV"), 10);
        if let Some(tv) = tvs.find_mut("192.168.1.50:5555") {
            tv.disabled_launchers
                .push("com.example.launcher".to_owned());
        }
        tvs.remember("192.168.1.50:5555", &info(None, "TV"), 20);
        assert_eq!(
            tvs.list()[0].disabled_launchers,
            vec!["com.example.launcher"]
        );
    }

    #[test]
    fn forget_should_remove_the_tv() {
        let mut tvs = SavedTvs::default();
        tvs.remember("192.168.1.50:5555", &info(None, "TV"), 10);
        assert!(tvs.forget("192.168.1.50:5555"));
    }

    #[test]
    fn forget_should_report_unknown_tv() {
        let mut tvs = SavedTvs::default();
        assert!(!tvs.forget("192.168.1.50:5555"));
    }

    #[test]
    fn moved_address_should_find_tv_by_serial_number() {
        let mut tvs = SavedTvs::default();
        tvs.remember("192.168.1.50:40001", &info(Some("ABC"), "TV"), 10);
        let services = [
            service("adb-XYZ-aaaa", "192.168.1.70:41000"),
            service("adb-ABC-bbbb", "192.168.1.80:42000"),
        ];
        assert_eq!(
            moved_address(&tvs.list()[0], &services),
            Some("192.168.1.80:42000")
        );
    }

    #[test]
    fn moved_address_should_fall_back_to_same_host() {
        let mut tvs = SavedTvs::default();
        tvs.remember("192.168.1.50:40001", &info(None, "TV"), 10);
        let services = [
            service("adb-XYZ-aaaa", "192.168.1.70:41000"),
            service("adb-QQQ-bbbb", "192.168.1.50:42000"),
        ];
        assert_eq!(
            moved_address(&tvs.list()[0], &services),
            Some("192.168.1.50:42000")
        );
    }

    #[test]
    fn moved_address_should_ignore_the_same_address() {
        let mut tvs = SavedTvs::default();
        tvs.remember("192.168.1.50:40001", &info(None, "TV"), 10);
        let services = [service("adb-QQQ-bbbb", "192.168.1.50:40001")];
        assert_eq!(moved_address(&tvs.list()[0], &services), None);
    }
}
