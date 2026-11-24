use serde::Serialize;
use std::fmt;

/// Errors returned to the frontend as `{ kind, message? }`, so the UI can pick a translated text by `kind`.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum AdbError {
    /// The bundled adb binary could not be started.
    Spawn(String),
    /// The address is not a valid `host[:port]`.
    InvalidAddress(String),
    /// The pairing code is not 6 digits.
    InvalidCode,
    /// `adb connect` reported a failure (refused, timed out, unreachable).
    ConnectFailed(String),
    /// `adb pair` reported a failure (wrong code, pairing dialog closed).
    PairFailed(String),
    /// The TV is not in the current device list.
    UnknownDevice(String),
    /// Any other adb command failure.
    Command(String),
}

impl fmt::Display for AdbError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(message) => write!(formatter, "could not start adb: {message}"),
            Self::InvalidAddress(address) => write!(formatter, "invalid address: {address}"),
            Self::InvalidCode => write!(formatter, "the pairing code must be 6 digits"),
            Self::ConnectFailed(message) => write!(formatter, "connect failed: {message}"),
            Self::PairFailed(message) => write!(formatter, "pairing failed: {message}"),
            Self::UnknownDevice(serial) => write!(formatter, "unknown device: {serial}"),
            Self::Command(message) => write!(formatter, "adb failed: {message}"),
        }
    }
}

impl std::error::Error for AdbError {}
