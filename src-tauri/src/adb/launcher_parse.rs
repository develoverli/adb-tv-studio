//! Pure parsers for the default launcher (the activity Android opens on HOME).

use super::apps_parse::package_name;
use std::collections::HashSet;

const SECTION_HOME: &str = "#home";
const SECTION_CURRENT: &str = "#current";
const SECTION_SYSTEM: &str = "#system";

/// One shell round trip: every HOME activity, the one HOME resolves to now, and the system packages.
pub const STATUS_SCRIPT: &str = "echo '#home'; \
cmd package query-activities --brief -a android.intent.action.MAIN -c android.intent.category.HOME; \
echo '#current'; \
cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.HOME; \
echo '#system'; pm list packages -s";

/// Android's chooser ("Open with…"): HOME resolves to it when no launcher is the default yet.
const CHOOSER_PACKAGE: &str = "android";

/// Boot-time placeholder launchers. Never offered and never disabled: Android needs them
/// while the device starts.
const FALLBACK_ACTIVITY: &str = "FallbackHome";

/// An app that can be the launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeApp {
    pub package: String,
    /// `package/activity`, as `cmd package set-home-activity` expects.
    pub component: String,
    pub system: bool,
}

/// Parsed [`STATUS_SCRIPT`] output.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HomeStatus {
    /// Package HOME opens now; `None` when Android would ask (no default) or did not say.
    pub current: Option<String>,
    pub apps: Vec<HomeApp>,
}

/// `package/activity` tokens on a line. Both parts are checked, so the component is safe to pass
/// as a word to the TV's shell (no `$`, quotes or spaces).
fn components(line: &str) -> impl Iterator<Item = (&str, &str)> {
    line.split_whitespace().filter_map(|token| {
        let (package, activity) = token.split_once('/')?;
        let valid_activity = !activity.is_empty()
            && activity.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '.' | '_')
            });
        if !valid_activity {
            return None;
        }
        package_name(package).ok().map(|package| (package, token))
    })
}

pub fn home_status(output: &str) -> HomeStatus {
    let mut section = "";
    let mut found: Vec<(String, String)> = Vec::new();
    let mut current = None;
    let mut system = HashSet::new();

    for line in output.lines().map(str::trim) {
        if line.starts_with('#') {
            section = line;
            continue;
        }
        match section {
            SECTION_HOME => {
                for (package, component) in components(line) {
                    let fallback = component.ends_with(FALLBACK_ACTIVITY);
                    if !fallback && !found.iter().any(|(known, _)| known == package) {
                        found.push((package.to_owned(), component.to_owned()));
                    }
                }
            }
            SECTION_CURRENT => {
                // `--brief` prints a "priority=..." line, then `package/activity`. The chooser
                // belongs to package "android", which `package_name` rejects (no dot) anyway.
                if let Some((package, _)) = components(line).last()
                    && package != CHOOSER_PACKAGE
                {
                    current = Some(package.to_owned());
                }
            }
            SECTION_SYSTEM => {
                if let Some(package) = line.strip_prefix("package:") {
                    system.insert(package.to_owned());
                }
            }
            _ => {}
        }
    }

    let apps = found
        .into_iter()
        .map(|(package, component)| HomeApp {
            system: system.contains(&package),
            package,
            component,
        })
        .collect();
    HomeStatus { current, apps }
}

/// Launchers to disable so `chosen` is the only one left: the other system launchers.
/// User-installed launchers are left alone; the user can uninstall them.
pub fn stock_launchers<'a>(status: &'a HomeStatus, chosen: &str) -> Vec<&'a str> {
    status
        .apps
        .iter()
        .filter(|app| app.system && app.package != chosen)
        .map(|app| app.package.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const OUTPUT: &str = "#home
priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=false
com.google.android.apps.tv.launcherx/.home.HomeActivity
priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=false
lol.develover.tvlauncher/.MainActivity
priority=-1000 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=false
com.android.tv.settings/.FallbackHome
#current
priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=true
com.google.android.apps.tv.launcherx/.home.HomeActivity
#system
package:com.google.android.apps.tv.launcherx
package:com.android.tv.settings
";

    #[test]
    fn home_status_should_list_launchers_without_fallback() {
        let packages: Vec<String> = home_status(OUTPUT)
            .apps
            .into_iter()
            .map(|app| app.package)
            .collect();
        assert_eq!(
            packages,
            vec![
                "com.google.android.apps.tv.launcherx",
                "lol.develover.tvlauncher"
            ]
        );
    }

    #[test]
    fn home_status_should_keep_the_component() {
        assert_eq!(
            home_status(OUTPUT).apps[1].component,
            "lol.develover.tvlauncher/.MainActivity"
        );
    }

    #[test]
    fn home_status_should_mark_system_launchers() {
        let systems: Vec<bool> = home_status(OUTPUT)
            .apps
            .iter()
            .map(|app| app.system)
            .collect();
        assert_eq!(systems, vec![true, false]);
    }

    #[test]
    fn home_status_should_read_current_launcher() {
        assert_eq!(
            home_status(OUTPUT).current.as_deref(),
            Some("com.google.android.apps.tv.launcherx")
        );
    }

    #[test]
    fn home_status_should_treat_chooser_as_no_default() {
        let output = "#current
priority=0 preferredOrder=0 match=0x0 specificIndex=-1 isDefault=false
android/com.android.internal.app.ResolverActivity
";
        assert_eq!(home_status(output).current, None);
    }

    #[test]
    fn stock_launchers_should_skip_chosen_and_user_apps() {
        let status = home_status(OUTPUT);
        assert_eq!(
            stock_launchers(&status, "lol.develover.tvlauncher"),
            vec!["com.google.android.apps.tv.launcherx"]
        );
    }

    #[test]
    fn stock_launchers_should_be_empty_when_chosen_is_the_only_system_one() {
        let status = home_status(OUTPUT);
        assert_eq!(
            stock_launchers(&status, "com.google.android.apps.tv.launcherx"),
            Vec::<&str>::new()
        );
    }
}
