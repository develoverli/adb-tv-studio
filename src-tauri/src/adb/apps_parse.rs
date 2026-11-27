//! Pure parsers for the Apps tab: package lists, launcher activities, `pm` results and APK icon paths.

use super::AdbError;
use std::collections::HashSet;

/// Longest package name Android accepts.
const MAX_PACKAGE_LEN: usize = 255;

/// Section markers printed by [`LIST_SCRIPT`] between the commands it runs.
const SECTION_ALL: &str = "#all";
const SECTION_SYSTEM: &str = "#system";
const SECTION_DISABLED: &str = "#disabled";
const SECTION_LAUNCHER: &str = "#launcher";
const SECTION_HOME: &str = "#home";

/// One shell round trip for the whole list: every package with its version code, the system and
/// disabled subsets, the activities the TV launcher (or a phone launcher) shows, and the apps that
/// can be the launcher themselves (HOME).
pub const LIST_SCRIPT: &str = "echo '#all'; pm list packages --show-versioncode; \
echo '#system'; pm list packages -s; \
echo '#disabled'; pm list packages -d; \
echo '#launcher'; \
cmd package query-activities --brief -a android.intent.action.MAIN -c android.intent.category.LEANBACK_LAUNCHER; \
cmd package query-activities --brief -a android.intent.action.MAIN -c android.intent.category.LAUNCHER; \
echo '#home'; \
cmd package query-activities --brief -a android.intent.action.MAIN -c android.intent.category.HOME";

/// A package as listed by [`LIST_SCRIPT`].
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent package flags, mirrored one to one by the frontend"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEntry {
    pub package: String,
    pub version_code: Option<String>,
    pub system: bool,
    pub disabled: bool,
    pub launcher: bool,
    /// Can be the default launcher (has a HOME activity).
    pub home: bool,
}

/// Validates an Android package name (`com.example.app`). Also keeps it safe as a shell word and a file name.
pub fn package_name(input: &str) -> Result<&str, AdbError> {
    let valid = !input.is_empty()
        && input.len() <= MAX_PACKAGE_LEN
        && input.contains('.')
        && !input.starts_with('.')
        && input
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_'));
    if valid {
        Ok(input)
    } else {
        Err(AdbError::InvalidPackage(input.to_owned()))
    }
}

/// Validates a version code (digits only), used as part of cache file names.
pub fn version_code(input: &str) -> Option<&str> {
    (!input.is_empty()
        && input.len() <= 20
        && input.chars().all(|character| character.is_ascii_digit()))
    .then_some(input)
}

/// Parses the output of [`LIST_SCRIPT`], sorted by package name.
pub fn package_list(output: &str) -> Vec<PackageEntry> {
    let mut section = "";
    let mut all: Vec<(String, Option<String>)> = Vec::new();
    let mut system = HashSet::new();
    let mut disabled = HashSet::new();
    let mut launcher = HashSet::new();
    let mut home = HashSet::new();

    for line in output.lines().map(str::trim) {
        if line.starts_with('#') {
            section = line;
            continue;
        }
        match section {
            SECTION_ALL => {
                let mut fields = line.split_whitespace();
                let Some(package) = fields
                    .next()
                    .and_then(|field| field.strip_prefix("package:"))
                else {
                    continue;
                };
                if package_name(package).is_err() {
                    continue;
                }
                let code = fields
                    .find_map(|field| field.strip_prefix("versionCode:"))
                    .and_then(version_code)
                    .map(str::to_owned);
                all.push((package.to_owned(), code));
            }
            SECTION_SYSTEM | SECTION_DISABLED => {
                if let Some(package) = line
                    .strip_prefix("package:")
                    .filter(|package| package_name(package).is_ok())
                {
                    let target = if section == SECTION_SYSTEM {
                        &mut system
                    } else {
                        &mut disabled
                    };
                    target.insert(package.to_owned());
                }
            }
            SECTION_LAUNCHER | SECTION_HOME => {
                let target = if section == SECTION_HOME {
                    &mut home
                } else {
                    &mut launcher
                };
                // `--brief` prints "package/activity" lines between "priority=..." lines.
                for token in line.split_whitespace() {
                    if let Some((package, _)) = token.split_once('/')
                        && package_name(package).is_ok()
                    {
                        target.insert(package.to_owned());
                    }
                }
            }
            _ => {}
        }
    }

    let mut entries: Vec<PackageEntry> = all
        .into_iter()
        .map(|(package, version_code)| PackageEntry {
            system: system.contains(&package),
            disabled: disabled.contains(&package),
            launcher: launcher.contains(&package),
            home: home.contains(&package),
            package,
            version_code,
        })
        .collect();
    entries.sort_by(|left, right| left.package.cmp(&right.package));
    entries
}

/// Picks the APK path from `pm path` output: `base.apk` when the app is split, otherwise the only one.
pub fn base_apk_path(output: &str) -> Option<String> {
    let paths: Vec<&str> = output
        .lines()
        .filter_map(|line| line.trim().strip_prefix("package:"))
        .filter(|path| path.starts_with('/') && has_extension(path, &["apk"]))
        .collect();
    paths
        .iter()
        .find(|path| path.ends_with("/base.apk"))
        .or_else(|| paths.first())
        .map(|path| (*path).to_owned())
}

/// Interprets output from `pm`, `am` and `monkey`. They often exit 0 on failure, so the text decides.
pub fn command_outcome(output: &str) -> Result<(), AdbError> {
    let text = output.trim();
    let failed = [
        "Failure",
        "Error",
        "Exception",
        "No activities found",
        "Unknown package",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    if failed {
        Err(AdbError::Command(text.to_owned()))
    } else {
        Ok(())
    }
}

/// Density folders from sharpest to softest. Unknown qualifiers rank last.
const DENSITIES: [&str; 6] = ["xxxhdpi", "xxhdpi", "xhdpi", "hdpi", "mdpi", "ldpi"];

fn density_rank(path: &str) -> usize {
    let folder = path.rsplit('/').nth(1).unwrap_or_default();
    DENSITIES
        .iter()
        .position(|density| folder.split('-').any(|part| part == *density))
        .unwrap_or(DENSITIES.len())
}

/// Case-insensitive extension check.
fn has_extension(path: &str, extensions: &[&str]) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extensions
                .iter()
                .any(|wanted| extension.eq_ignore_ascii_case(wanted))
        })
}

fn is_raster(path: &str) -> bool {
    has_extension(path, &["png", "webp", "jpg"])
}

/// Best raster image for a resolved icon or banner path.
///
/// `resolved` may point at one density, or at an XML (adaptive icon, vector) that cannot be drawn
/// here; then the sharpest PNG/WebP with the same file name in any `mipmap-*`/`drawable-*` folder
/// is used, falling back to an adaptive icon's `_foreground` layer.
pub fn best_raster<'a>(resolved: &str, names: &[&'a str]) -> Option<&'a str> {
    let file = resolved.rsplit('/').next()?;
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    let pick = |wanted: &str| {
        names
            .iter()
            .copied()
            .filter(|name| name.starts_with("res/") && is_raster(name))
            .filter(|name| {
                let name_file = name.rsplit('/').next().unwrap_or_default();
                name_file
                    .rsplit_once('.')
                    .is_some_and(|(name_stem, _)| name_stem == wanted)
            })
            .min_by_key(|name| density_rank(name))
    };
    pick(stem).or_else(|| pick(&format!("{stem}_foreground")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST_OUTPUT: &str = "#all\n\
package:com.netflix.ninja versionCode:1234\n\
package:com.android.providers.tv versionCode:30\n\
package:com.example.disabled versionCode:7\n\
#system\n\
package:com.android.providers.tv\n\
package:com.netflix.ninja\n\
#disabled\n\
package:com.example.disabled\n\
#launcher\n\
priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=false\n\
  com.netflix.ninja/.MainActivity\n\
#home\n\
priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=false\n\
  com.example.disabled/.Home\n";

    #[test]
    fn package_list_should_merge_sections() {
        let entries = package_list(LIST_OUTPUT);
        let netflix = entries
            .iter()
            .find(|entry| entry.package == "com.netflix.ninja");
        assert_eq!(
            netflix,
            Some(&PackageEntry {
                package: "com.netflix.ninja".to_owned(),
                version_code: Some("1234".to_owned()),
                system: true,
                disabled: false,
                launcher: true,
                home: false,
            })
        );
    }

    #[test]
    fn package_list_should_mark_home_apps() {
        let entries = package_list(LIST_OUTPUT);
        assert!(
            entries
                .iter()
                .any(|entry| entry.package == "com.example.disabled" && entry.home)
        );
    }

    #[test]
    fn package_list_should_mark_disabled() {
        let entries = package_list(LIST_OUTPUT);
        assert!(
            entries
                .iter()
                .any(|entry| entry.package == "com.example.disabled" && entry.disabled)
        );
    }

    #[test]
    fn package_list_should_sort_by_package() {
        let names: Vec<String> = package_list(LIST_OUTPUT)
            .into_iter()
            .map(|entry| entry.package)
            .collect();
        assert_eq!(
            names,
            vec![
                "com.android.providers.tv",
                "com.example.disabled",
                "com.netflix.ninja"
            ]
        );
    }

    #[test]
    fn package_name_should_reject_shell_characters() {
        assert!(package_name("com.app;reboot").is_err());
    }

    #[test]
    fn package_name_should_reject_path_traversal() {
        assert!(package_name("../etc").is_err());
    }

    #[test]
    fn package_name_should_accept_underscores() {
        assert!(package_name("com.my_app.tv").is_ok());
    }

    #[test]
    fn version_code_should_reject_non_digits() {
        assert_eq!(version_code("12a"), None);
    }

    #[test]
    fn base_apk_path_should_prefer_base_of_split_app() {
        let output = "package:/data/app/~~a==/com.x-b==/split_config.xhdpi.apk\npackage:/data/app/~~a==/com.x-b==/base.apk\n";
        assert_eq!(
            base_apk_path(output),
            Some("/data/app/~~a==/com.x-b==/base.apk".to_owned())
        );
    }

    #[test]
    fn base_apk_path_should_accept_single_apk() {
        assert_eq!(
            base_apk_path("package:/system/app/Tv/Tv.apk\r\n"),
            Some("/system/app/Tv/Tv.apk".to_owned())
        );
    }

    #[test]
    fn command_outcome_should_fail_on_pm_failure() {
        assert!(command_outcome("Failure [DELETE_FAILED_INTERNAL_ERROR]").is_err());
    }

    #[test]
    fn command_outcome_should_fail_when_monkey_finds_nothing() {
        assert!(command_outcome("** No activities found to run, monkey aborted.").is_err());
    }

    #[test]
    fn command_outcome_should_accept_success() {
        assert!(command_outcome("Success").is_ok());
    }

    #[test]
    fn best_raster_should_pick_sharpest_density_for_adaptive_icon() {
        let names = [
            "res/mipmap-anydpi-v26/ic_launcher.xml",
            "res/mipmap-hdpi-v4/ic_launcher.png",
            "res/mipmap-xxxhdpi-v4/ic_launcher.webp",
            "res/mipmap-xhdpi-v4/ic_launcher.png",
        ];
        assert_eq!(
            best_raster("res/mipmap-anydpi-v26/ic_launcher.xml", &names),
            Some("res/mipmap-xxxhdpi-v4/ic_launcher.webp")
        );
    }

    #[test]
    fn best_raster_should_fall_back_to_foreground_layer() {
        let names = [
            "res/mipmap-anydpi-v26/ic_launcher.xml",
            "res/mipmap-xxhdpi-v4/ic_launcher_foreground.png",
        ];
        assert_eq!(
            best_raster("res/mipmap-anydpi-v26/ic_launcher.xml", &names),
            Some("res/mipmap-xxhdpi-v4/ic_launcher_foreground.png")
        );
    }

    #[test]
    fn best_raster_should_return_none_without_rasters() {
        assert_eq!(
            best_raster("res/drawable/banner.xml", &["res/drawable/banner.xml"]),
            None
        );
    }
}
