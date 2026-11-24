//! Pure parsers for adb CLI output and user input. No I/O, so every branch is unit tested.

use super::AdbError;
use serde::Serialize;
use std::net::IpAddr;

/// mDNS service type adb uses for TLS connections (Android 11+ wireless debugging).
const MDNS_CONNECT: &str = "_adb-tls-connect._tcp";
/// mDNS service type a TV announces only while its "Pair device with code" dialog is open.
const MDNS_PAIRING: &str = "_adb-tls-pairing._tcp";
/// Length of the code shown in the TV's pairing dialog.
const PAIRING_CODE_LEN: usize = 6;
/// Length of the hex size prefix on every `track-devices` frame.
const FRAME_HEADER_LEN: usize = 4;

/// Connection state shown in the UI. adb has more states; they collapse into these three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceStatus {
    Connected,
    Offline,
    Unauthorized,
}

/// Maps an adb state word (`device`, `offline`, `unauthorized`, `authorizing`, ...) to a UI status.
pub fn device_status(state: &str) -> DeviceStatus {
    match state {
        "device" => DeviceStatus::Connected,
        "unauthorized" | "authorizing" => DeviceStatus::Unauthorized,
        _ => DeviceStatus::Offline,
    }
}

/// Parses `serial<TAB>state` lines, as printed by `adb devices` and each `track-devices` frame.
pub fn device_list(text: &str) -> Vec<(String, DeviceStatus)> {
    text.lines()
        .filter(|line| !line.starts_with("List of devices"))
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let serial = fields.next()?;
            let state = fields.next()?;
            Some((serial.to_owned(), device_status(state)))
        })
        .collect()
}

/// True for TVs reached over the network (`host:port` or an mDNS name); false for USB serials and emulators.
pub fn is_network_serial(serial: &str) -> bool {
    if serial.contains(MDNS_CONNECT) {
        return true;
    }
    serial.rsplit_once(':').is_some_and(|(host, port)| {
        port.parse::<u16>().is_ok() && (host.parse::<IpAddr>().is_ok() || host.contains('.'))
    })
}

/// Splits the stream of `adb track-devices` into frames.
///
/// Each frame is a 4-digit hex length followed by that many bytes of `serial<TAB>state` lines.
/// An empty device list is the bare frame `0000`, with no newline, so line-based reading is not enough.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
}

impl FrameDecoder {
    /// Adds a chunk of raw stdout and returns every frame it completes.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();
        while let Some(header) = self.buffer.get(..FRAME_HEADER_LEN) {
            let Some(length) = std::str::from_utf8(header)
                .ok()
                .and_then(|hex| usize::from_str_radix(hex, 16).ok())
            else {
                // Not a frame header: the stream is out of sync (or adb printed an error). Drop it and resync.
                self.buffer.clear();
                break;
            };
            let end = FRAME_HEADER_LEN + length;
            let Some(payload) = self.buffer.get(FRAME_HEADER_LEN..end) else {
                break;
            };
            frames.push(String::from_utf8_lossy(payload).into_owned());
            self.buffer.drain(..end);
        }
        frames
    }
}

/// Which mDNS service a discovered entry announces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdnsKind {
    Connect,
    Pairing,
}

/// One line of `adb mdns services`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsService {
    pub instance: String,
    pub kind: MdnsKind,
    pub address: String,
}

/// Parses `adb mdns services`: `instance<TAB>service<TAB>ip:port` per line, after a header line.
pub fn mdns_services(text: &str) -> Vec<MdnsService> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let instance = fields.next()?;
            let service = fields.next()?.trim_end_matches('.');
            let address = fields.next()?;
            let kind = match service {
                MDNS_CONNECT => MdnsKind::Connect,
                MDNS_PAIRING => MdnsKind::Pairing,
                _ => return None,
            };
            Some(MdnsService {
                instance: instance.to_owned(),
                kind,
                address: address.to_owned(),
            })
        })
        .collect()
}

/// Result of `adb connect` when it did reach the TV.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectStatus {
    /// Connected, or already connected.
    Connected,
    /// Reached, but the TV still has to accept the "Allow debugging?" dialog.
    Unauthorized,
}

/// Interprets `adb connect` output. adb exits 0 on some failures, so the text decides.
pub fn connect_outcome(output: &str) -> Result<ConnectStatus, AdbError> {
    let text = output.trim();
    let lower = text.to_lowercase();
    if lower.contains("failed to authenticate") {
        Ok(ConnectStatus::Unauthorized)
    } else if lower.contains("connected to")
        && !lower.contains("failed")
        && !lower.contains("cannot")
    {
        Ok(ConnectStatus::Connected)
    } else {
        Err(AdbError::ConnectFailed(text.to_owned()))
    }
}

/// Interprets `adb pair` output: success is `Successfully paired to host:port [guid=...]`.
pub fn pair_outcome(output: &str) -> Result<(), AdbError> {
    let text = output.trim();
    if text
        .lines()
        .any(|line| line.starts_with("Successfully paired"))
    {
        Ok(())
    } else {
        Err(AdbError::PairFailed(text.to_owned()))
    }
}

/// Facts read from the TV once it is connected. `None` when the TV did not report a value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceInfo {
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub android_version: Option<String>,
}

/// Shell script whose output `device_info` parses: one value per line, in this order.
pub const INFO_SCRIPT: &str = "getprop ro.product.manufacturer; getprop ro.product.model; \
getprop ro.build.version.release; settings get global device_name";

/// Parses the output of [`INFO_SCRIPT`].
pub fn device_info(output: &str) -> DeviceInfo {
    let mut values = output.lines().map(|line| {
        let value = line.trim();
        (!value.is_empty() && value != "null").then(|| value.to_owned())
    });
    let mut next = || values.next().flatten();
    DeviceInfo {
        manufacturer: next(),
        model: next(),
        android_version: next(),
        name: next(),
    }
}

/// Validates `host[:port]` typed by the user and returns `host:port`.
///
/// `default_port` fills a missing port; with `None` the port is required.
/// Hosts are IPs or hostnames (letters, digits, `.` and `-`, not starting with `-`),
/// which also keeps the value from being read as an adb flag.
pub fn address(input: &str, default_port: Option<u16>) -> Result<String, AdbError> {
    let invalid = || AdbError::InvalidAddress(input.to_owned());
    let input_trimmed = input.trim();
    let (host, port) = match input_trimmed.rsplit_once(':') {
        Some((host, port)) => (
            host,
            port.parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .ok_or_else(invalid)?,
        ),
        None => (input_trimmed, default_port.ok_or_else(invalid)?),
    };
    let valid_host = !host.is_empty()
        && !host.starts_with('-')
        && host
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'));
    if valid_host {
        Ok(format!("{host}:{port}"))
    } else {
        Err(invalid())
    }
}

/// Validates the 6-digit pairing code.
pub fn pairing_code(input: &str) -> Result<String, AdbError> {
    let code = input.trim();
    if code.len() == PAIRING_CODE_LEN && code.chars().all(|character| character.is_ascii_digit()) {
        Ok(code.to_owned())
    } else {
        Err(AdbError::InvalidCode)
    }
}

/// Host part of `host:port`.
pub fn host_of(address: &str) -> &str {
    address.rsplit_once(':').map_or(address, |(host, _)| host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_list_should_parse_states_and_skip_header() {
        let text = "List of devices attached\n192.168.1.50:5555\tdevice\n192.168.1.73:5555\tunauthorized\n10.0.0.2:5555\toffline\n";
        assert_eq!(
            device_list(text),
            vec![
                ("192.168.1.50:5555".to_owned(), DeviceStatus::Connected),
                ("192.168.1.73:5555".to_owned(), DeviceStatus::Unauthorized),
                ("10.0.0.2:5555".to_owned(), DeviceStatus::Offline),
            ]
        );
    }

    #[test]
    fn device_status_should_treat_authorizing_as_unauthorized() {
        assert_eq!(device_status("authorizing"), DeviceStatus::Unauthorized);
    }

    #[test]
    fn device_status_should_treat_unknown_states_as_offline() {
        assert_eq!(device_status("no permissions"), DeviceStatus::Offline);
    }

    #[test]
    fn is_network_serial_should_accept_ip_and_port() {
        assert!(is_network_serial("192.168.1.50:5555"));
    }

    #[test]
    fn is_network_serial_should_accept_mdns_name() {
        assert!(is_network_serial("adb-1A2B3C-xYz._adb-tls-connect._tcp"));
    }

    #[test]
    fn is_network_serial_should_reject_usb_serial() {
        assert!(!is_network_serial("R58M12345AB"));
    }

    #[test]
    fn is_network_serial_should_reject_emulator() {
        assert!(!is_network_serial("emulator-5554"));
    }

    #[test]
    fn frame_decoder_should_split_back_to_back_frames() {
        let mut decoder = FrameDecoder::default();
        let frames = decoder.push(b"00000019192.168.1.50:5555\tdevice\n");
        assert_eq!(
            frames,
            vec![String::new(), "192.168.1.50:5555\tdevice\n".to_owned()]
        );
    }

    #[test]
    fn frame_decoder_should_wait_for_split_frames() {
        let mut decoder = FrameDecoder::default();
        let first = decoder.push(b"0019192.168.");
        let second = decoder.push(b"1.50:5555\tdevice\n");
        assert_eq!(
            (first.len(), second),
            (0, vec!["192.168.1.50:5555\tdevice\n".to_owned()])
        );
    }

    #[test]
    fn frame_decoder_should_resync_after_garbage() {
        let mut decoder = FrameDecoder::default();
        let garbage = decoder.push(b"* daemon started\n");
        let frames = decoder.push(b"0000");
        assert_eq!((garbage.len(), frames), (0, vec![String::new()]));
    }

    #[test]
    fn mdns_services_should_parse_connect_and_pairing() {
        let text = "List of discovered mdns services\n\
adb-1A2B-xYz\t_adb-tls-connect._tcp\t192.168.1.50:37011\n\
adb-1A2B-xYz\t_adb-tls-pairing._tcp.\t192.168.1.50:41234\n";
        assert_eq!(
            mdns_services(text),
            vec![
                MdnsService {
                    instance: "adb-1A2B-xYz".to_owned(),
                    kind: MdnsKind::Connect,
                    address: "192.168.1.50:37011".to_owned(),
                },
                MdnsService {
                    instance: "adb-1A2B-xYz".to_owned(),
                    kind: MdnsKind::Pairing,
                    address: "192.168.1.50:41234".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn mdns_services_should_ignore_other_services() {
        assert!(mdns_services("adb-1A2B\t_adb._tcp\t192.168.1.50:5555\n").is_empty());
    }

    #[test]
    fn connect_outcome_should_accept_connected() {
        assert_eq!(
            connect_outcome("connected to 192.168.1.50:5555\n").ok(),
            Some(ConnectStatus::Connected)
        );
    }

    #[test]
    fn connect_outcome_should_accept_already_connected() {
        assert_eq!(
            connect_outcome("already connected to 192.168.1.50:5555").ok(),
            Some(ConnectStatus::Connected)
        );
    }

    #[test]
    fn connect_outcome_should_report_unauthorized() {
        assert_eq!(
            connect_outcome("failed to authenticate to 192.168.1.50:5555").ok(),
            Some(ConnectStatus::Unauthorized)
        );
    }

    #[test]
    fn connect_outcome_should_fail_on_refused() {
        assert!(matches!(
            connect_outcome("failed to connect to '192.168.1.50:5555': Connection refused"),
            Err(AdbError::ConnectFailed(_))
        ));
    }

    #[test]
    fn connect_outcome_should_fail_on_cannot_connect() {
        assert!(connect_outcome("cannot connect to 192.168.1.50:5555: timed out").is_err());
    }

    #[test]
    fn pair_outcome_should_accept_success() {
        assert!(
            pair_outcome("Successfully paired to 192.168.1.50:41234 [guid=adb-1A2B-xYz]").is_ok()
        );
    }

    #[test]
    fn pair_outcome_should_fail_on_wrong_code() {
        assert!(pair_outcome("Failed: Wrong password or connection was interrupted.").is_err());
    }

    #[test]
    fn device_info_should_read_values_in_order() {
        let info = device_info("Google\nChromecast\n14\nLiving room\n");
        assert_eq!(
            info,
            DeviceInfo {
                name: Some("Living room".to_owned()),
                manufacturer: Some("Google".to_owned()),
                model: Some("Chromecast".to_owned()),
                android_version: Some("14".to_owned()),
            }
        );
    }

    #[test]
    fn device_info_should_drop_null_name() {
        assert_eq!(device_info("Xiaomi\nMiBox\n9\nnull\n").name, None);
    }

    #[test]
    fn address_should_add_default_port() {
        assert_eq!(
            address(" 192.168.1.50 ", Some(5555)).ok(),
            Some("192.168.1.50:5555".to_owned())
        );
    }

    #[test]
    fn address_should_keep_given_port() {
        assert_eq!(
            address("tv.local:37011", Some(5555)).ok(),
            Some("tv.local:37011".to_owned())
        );
    }

    #[test]
    fn address_should_require_port_without_default() {
        assert!(address("192.168.1.50", None).is_err());
    }

    #[test]
    fn address_should_reject_flag_like_host() {
        assert!(address("-P:5555", Some(5555)).is_err());
    }

    #[test]
    fn address_should_reject_spaces() {
        assert!(address("192.168.1.50 shell:5555", None).is_err());
    }

    #[test]
    fn address_should_reject_port_zero() {
        assert!(address("192.168.1.50:0", None).is_err());
    }

    #[test]
    fn pairing_code_should_accept_six_digits() {
        assert_eq!(pairing_code("123456").ok(), Some("123456".to_owned()));
    }

    #[test]
    fn pairing_code_should_reject_letters() {
        assert!(pairing_code("12a456").is_err());
    }

    #[test]
    fn host_of_should_strip_port() {
        assert_eq!(host_of("192.168.1.50:37011"), "192.168.1.50");
    }
}
