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
    /// Text for the TV is empty, too long or not printable ASCII (`input text` cannot type it).
    InvalidText,
    /// Not a valid Android package name.
    InvalidPackage(String),
    /// Reading the app's APK (pull or parse) failed.
    ApkRead(String),
    /// `adb install` failed; carries Android's code, e.g. `INSTALL_FAILED_VERSION_DOWNGRADE`.
    InstallFailed(String),
    /// There is no screen capture yet to save.
    NoCapture,
    /// Writing the saved capture to disk failed.
    Save(String),
    /// The package has no HOME activity, so it cannot be the launcher.
    NotALauncher(String),
    /// Android accepted the launcher change but HOME still opens another one (usually the stock
    /// launcher overriding it); disabling the stock launcher is the next step.
    LauncherNotApplied,
    /// Disabling the stock launcher needs another launcher set first, or the TV would have none.
    NoCustomLauncher,
    /// The TV is not saved, so what this app changes on it cannot be recorded for undoing later.
    NotSaved,
    /// A theme file or zip does not follow the theme contract; says what is wrong.
    InvalidTheme(String),
    /// Reading or writing themes on this computer failed.
    ThemeStorage(String),
    /// tv-launcher is not installed on the TV.
    LauncherMissing,
    /// The installed tv-launcher is too old to read themes.
    LauncherOutdated,
    /// This build of the app does not include the tv-launcher APK.
    LauncherNotBundled,
    /// The TV has tv-launcher signed with another key, so it cannot be updated in place.
    LauncherSignatureMismatch,
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
            Self::InvalidText => write!(formatter, "text must be printable ASCII"),
            Self::InvalidPackage(package) => write!(formatter, "invalid package name: {package}"),
            Self::ApkRead(message) => write!(formatter, "could not read the APK: {message}"),
            Self::InstallFailed(code) => write!(formatter, "install failed: {code}"),
            Self::NoCapture => write!(formatter, "no screen capture to save"),
            Self::Save(message) => write!(formatter, "could not save the capture: {message}"),
            Self::NotALauncher(package) => write!(formatter, "{package} cannot be the launcher"),
            Self::LauncherNotApplied => write!(formatter, "HOME still opens another launcher"),
            Self::NoCustomLauncher => write!(formatter, "set another launcher first"),
            Self::NotSaved => write!(formatter, "the TV is not saved"),
            Self::InvalidTheme(problem) => write!(formatter, "invalid theme: {problem}"),
            Self::ThemeStorage(message) => write!(formatter, "theme storage failed: {message}"),
            Self::LauncherMissing => write!(formatter, "tv-launcher is not installed"),
            Self::LauncherOutdated => write!(formatter, "tv-launcher is too old for themes"),
            Self::LauncherNotBundled => write!(formatter, "this build has no tv-launcher APK"),
            Self::LauncherSignatureMismatch => {
                write!(
                    formatter,
                    "tv-launcher on the TV is signed with another key"
                )
            }
            Self::Command(message) => write!(formatter, "adb failed: {message}"),
        }
    }
}

impl std::error::Error for AdbError {}
