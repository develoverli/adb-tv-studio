//! Pure parsers for the Install tab: installed version from `dumpsys package`, install results,
//! and how a file compares with what the TV has.

use super::AdbError;
use serde::Serialize;
use std::path::Path;

/// File types the Install tab accepts: plain APKs and split containers.
pub const EXTENSIONS: [&str; 4] = ["apk", "xapk", "apks", "apkm"];

/// True for an accepted extension, case-insensitive.
pub fn is_installable(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            EXTENSIONS
                .iter()
                .any(|wanted| extension.eq_ignore_ascii_case(wanted))
        })
}

/// Version of a package installed on the TV.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledVersion {
    pub code: Option<u64>,
    pub name: Option<String>,
}

/// Reads `versionCode` and `versionName` for `package` from `dumpsys package <package>`.
/// `None` when the package is not installed.
pub fn installed_version(output: &str, package: &str) -> Option<InstalledVersion> {
    let header = format!("Package [{package}]");
    let mut lines = output.lines().skip_while(|line| !line.contains(&header));
    lines.next()?;
    let mut version = InstalledVersion {
        code: None,
        name: None,
    };
    for line in lines {
        let line = line.trim();
        // The next "Package [" block belongs to another package (or another user's copy).
        if line.starts_with("Package [") {
            break;
        }
        for field in line.split_whitespace() {
            if let Some(code) = field.strip_prefix("versionCode=") {
                version.code = version.code.or_else(|| code.parse().ok());
            }
        }
        if let Some(name) = line.strip_prefix("versionName=") {
            version.name = version.name.or_else(|| Some(name.to_owned()));
        }
    }
    Some(version)
}

/// How a file compares with the TV. Serialized as camelCase for the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallKind {
    New,
    Update,
    Same,
    Downgrade,
    /// Installed, but one of the version codes is unknown.
    Replace,
}

pub fn install_kind(file_code: Option<u64>, installed: Option<&InstalledVersion>) -> InstallKind {
    let Some(installed) = installed else {
        return InstallKind::New;
    };
    match (file_code, installed.code) {
        (Some(file), Some(current)) if file > current => InstallKind::Update,
        (Some(file), Some(current)) if file == current => InstallKind::Same,
        (Some(_), Some(_)) => InstallKind::Downgrade,
        _ => InstallKind::Replace,
    }
}

/// Interprets `adb install` / `install-multiple` output.
///
/// Failures look like `adb: failed to install x.apk: Failure [INSTALL_FAILED_VERSION_DOWNGRADE: ...]`;
/// the code inside the brackets is returned so the UI can explain it.
pub fn install_outcome(output: &str) -> Result<(), AdbError> {
    let text = output.trim();
    if let Some(start) = text.find("Failure [") {
        let rest = &text[start + "Failure [".len()..];
        let code: String = rest
            .chars()
            .take_while(|character| character.is_ascii_uppercase() || *character == '_')
            .collect();
        return Err(AdbError::InstallFailed(if code.is_empty() {
            rest.trim_end_matches(']').to_owned()
        } else {
            code
        }));
    }
    if text.lines().any(|line| line.trim() == "Success") {
        Ok(())
    } else {
        Err(AdbError::InstallFailed(text.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DUMPSYS: &str = "Packages:\n  Package [com.netflix.ninja] (a1b2c3):\n    userId=10050\n    \
versionCode=1234 minSdk=21 targetSdk=30\n    versionName=8.1.0\n    splits=[base]\n";

    #[test]
    fn installed_version_should_read_code_and_name() {
        assert_eq!(
            installed_version(DUMPSYS, "com.netflix.ninja"),
            Some(InstalledVersion {
                code: Some(1234),
                name: Some("8.1.0".to_owned()),
            })
        );
    }

    #[test]
    fn installed_version_should_be_none_when_missing() {
        assert_eq!(
            installed_version("Unable to find package: com.x.y\n", "com.x.y"),
            None
        );
    }

    #[test]
    fn installed_version_should_stop_at_next_package() {
        let output =
            "  Package [com.a.b] (1):\n  Package [com.c.d] (2):\n    versionCode=9 minSdk=21\n";
        assert_eq!(
            installed_version(output, "com.a.b"),
            Some(InstalledVersion {
                code: None,
                name: None,
            })
        );
    }

    #[test]
    fn install_kind_should_detect_update() {
        let installed = InstalledVersion {
            code: Some(10),
            name: None,
        };
        assert_eq!(
            install_kind(Some(11), Some(&installed)),
            InstallKind::Update
        );
    }

    #[test]
    fn install_kind_should_detect_downgrade() {
        let installed = InstalledVersion {
            code: Some(10),
            name: None,
        };
        assert_eq!(
            install_kind(Some(9), Some(&installed)),
            InstallKind::Downgrade
        );
    }

    #[test]
    fn install_kind_should_be_new_when_not_installed() {
        assert_eq!(install_kind(Some(1), None), InstallKind::New);
    }

    #[test]
    fn install_outcome_should_accept_success() {
        assert!(install_outcome("Performing Streamed Install\r\nSuccess\r\n").is_ok());
    }

    #[test]
    fn install_outcome_should_extract_failure_code() {
        let result = install_outcome(
            "adb: failed to install a.apk: Failure [INSTALL_FAILED_VERSION_DOWNGRADE: Downgrade detected]",
        );
        assert!(
            matches!(result, Err(AdbError::InstallFailed(code)) if code == "INSTALL_FAILED_VERSION_DOWNGRADE")
        );
    }

    #[test]
    fn install_outcome_should_fail_without_success() {
        assert!(install_outcome("adb: error: failed to read").is_err());
    }

    #[test]
    fn is_installable_should_accept_uppercase_xapk() {
        assert!(is_installable(Path::new("C:/Downloads/App.XAPK")));
    }

    #[test]
    fn is_installable_should_reject_zip() {
        assert!(!is_installable(Path::new("app.zip")));
    }
}
