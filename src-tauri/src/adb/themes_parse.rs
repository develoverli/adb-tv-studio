//! The tv-launcher theme contract (`theme.json`, schemaVersion 1) and its validation.
//!
//! Mirrors `schema/theme.schema.json` in the tv-launcher-themes repo: unknown fields are
//! rejected, enums are closed and every value is range-checked, so only data the launcher
//! understands ever reaches the TV.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;
/// Id the launcher keeps for its built-in look.
const RESERVED_ID: &str = "default";
const MAX_ID_LEN: usize = 40;
const MAX_NAME_LEN: usize = 40;
const MAX_AUTHOR_LEN: usize = 60;
const MAX_DESCRIPTION_LEN: usize = 200;
const MAX_ASSET_NAME_LEN: usize = 40;
const ASSET_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];
const MIN_COLUMNS: u8 = 3;
const MAX_COLUMNS: u8 = 6;
const MAX_ANGLE: u16 = 359;
const MAX_BLUR: u8 = 25;
const MAX_VEIL: f32 = 0.9;
pub const THEME_FILE: &str = "theme.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    Classic,
    Rows,
    Grid,
    Minimal,
    Sidebar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Material {
    Solid,
    Glass,
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    Square,
    Rounded,
    Round,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Focus {
    Scale,
    Lift,
    Glow,
    Outline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Font {
    Outfit,
    Sans,
    Serif,
    Mono,
    Condensed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    None,
    Smooth,
    Springy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Labels {
    Focus,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClockSize {
    Small,
    Normal,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundType {
    Solid,
    Gradient,
    Image,
    Photos,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Colors {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// A colour, or `auto` for the focused app's banner colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Background {
    #[serde(rename = "type")]
    pub kind: BackgroundType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colors: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blur: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub veil: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Clock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<ClockSize>,
}

/// A `theme.json`. Serialized back unchanged to the frontend for previews.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeFile {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub layout: Layout,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colors: Option<Colors>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Background>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<Material>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<Focus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<Font>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion: Option<Motion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<Labels>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock: Option<Clock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glow: Option<bool>,
}

impl ThemeFile {
    /// Image files the theme refers to (only the background image in version 1).
    pub fn assets(&self) -> Vec<&str> {
        self.background
            .as_ref()
            .and_then(|background| background.image.as_deref())
            .into_iter()
            .collect()
    }
}

/// Valid theme id: also its folder name on the TV and on disk, and a safe shell word.
pub fn is_theme_id(id: &str) -> bool {
    let mut characters = id.chars();
    let starts_well = characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit());
    starts_well
        && id.len() <= MAX_ID_LEN
        && id != RESERVED_ID
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

/// Valid image file name inside a theme folder.
pub fn is_asset_name(name: &str) -> bool {
    let Some((stem, extension)) = name.rsplit_once('.') else {
        return false;
    };
    !stem.is_empty()
        && name.len() <= MAX_ASSET_NAME_LEN + 5
        && stem.len() <= MAX_ASSET_NAME_LEN
        && stem.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '_' | '-')
        })
        && ASSET_EXTENSIONS.contains(&extension)
}

fn is_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

fn is_version(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty() && part.chars().all(|character| character.is_ascii_digit())
        })
}

fn check(condition: bool, problem: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(problem.to_owned())
    }
}

fn check_text(value: Option<&str>, max: usize, field: &str) -> Result<(), String> {
    check(
        value.is_none_or(|text| text.chars().count() <= max && !text.contains(['\n', '\r'])),
        &format!("{field} is too long or has line breaks"),
    )
}

/// Parses and validates a `theme.json`. The error says what is wrong, in English.
pub fn parse_theme(text: &str) -> Result<ThemeFile, String> {
    let theme: ThemeFile = serde_json::from_str(text).map_err(|error| error.to_string())?;
    validate(&theme)?;
    Ok(theme)
}

fn validate(theme: &ThemeFile) -> Result<(), String> {
    check(
        theme.schema_version == SCHEMA_VERSION,
        "unsupported schemaVersion",
    )?;
    check(is_theme_id(&theme.id), "invalid id")?;
    let name_length = theme.name.chars().count();
    check(
        name_length > 0 && name_length <= MAX_NAME_LEN,
        "name must be 1 to 40 characters",
    )?;
    check_text(Some(&theme.name), MAX_NAME_LEN, "name")?;
    check_text(theme.author.as_deref(), MAX_AUTHOR_LEN, "author")?;
    check_text(
        theme.description.as_deref(),
        MAX_DESCRIPTION_LEN,
        "description",
    )?;
    check(
        theme.version.as_deref().is_none_or(is_version),
        "version must look like 1.0.0",
    )?;
    check(
        theme
            .columns
            .is_none_or(|columns| (MIN_COLUMNS..=MAX_COLUMNS).contains(&columns)),
        "columns must be 3 to 6",
    )?;
    if let Some(colors) = &theme.colors {
        for color in [&colors.background, &colors.surface, &colors.text]
            .into_iter()
            .flatten()
        {
            check(is_color(color), "colors must look like #RRGGBB")?;
        }
        check(
            colors
                .accent
                .as_deref()
                .is_none_or(|accent| accent == "auto" || is_color(accent)),
            "accent must be #RRGGBB or auto",
        )?;
    }
    if let Some(background) = &theme.background {
        validate_background(background)?;
    }
    Ok(())
}

fn validate_background(background: &Background) -> Result<(), String> {
    if let Some(colors) = &background.colors {
        check(
            colors.len() == 2 && colors.iter().all(|color| is_color(color)),
            "background.colors must be two #RRGGBB colours",
        )?;
    }
    check(
        background.kind != BackgroundType::Gradient || background.colors.is_some(),
        "a gradient background needs colors",
    )?;
    check(
        background.angle.is_none_or(|angle| angle <= MAX_ANGLE),
        "background.angle must be 0 to 359",
    )?;
    check(
        background.blur.is_none_or(|blur| blur <= MAX_BLUR),
        "background.blur must be 0 to 25",
    )?;
    check(
        background
            .veil
            .is_none_or(|veil| (0.0..=MAX_VEIL).contains(&veil)),
        "background.veil must be 0 to 0.9",
    )?;
    check(
        background.image.as_deref().is_none_or(is_asset_name),
        "background.image must be a png, jpg or webp file name",
    )?;
    check(
        background.kind != BackgroundType::Image || background.image.is_some(),
        "an image background needs image",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r##"{
        "$schema": "https://example.com/theme.schema.json",
        "schemaVersion": 1,
        "id": "vidrio",
        "name": "Vidrio",
        "layout": "rows",
        "colors": { "background": "#0A0D14", "accent": "auto" },
        "background": { "type": "gradient", "colors": ["#1B2A4A", "#0A0D14"], "angle": 180 },
        "material": "glass",
        "columns": 4
    }"##;

    fn with(field: &str, value: &str) -> String {
        VALID.replacen(
            "\"columns\": 4",
            &format!("\"columns\": 4, \"{field}\": {value}"),
            1,
        )
    }

    #[test]
    fn parse_theme_should_accept_a_valid_theme() {
        assert_eq!(
            parse_theme(VALID).map(|theme| theme.id),
            Ok("vidrio".to_owned())
        );
    }

    #[test]
    fn parse_theme_should_reject_unknown_fields() {
        assert!(parse_theme(&with("script", "\"alert(1)\"")).is_err());
    }

    #[test]
    fn parse_theme_should_reject_unknown_layout() {
        assert!(parse_theme(&VALID.replace("\"rows\"", "\"carousel\"")).is_err());
    }

    #[test]
    fn parse_theme_should_reject_bad_colors() {
        assert!(parse_theme(&VALID.replace("#0A0D14\", \"accent", "red\", \"accent")).is_err());
    }

    #[test]
    fn parse_theme_should_reject_too_many_columns() {
        assert!(parse_theme(&VALID.replace("\"columns\": 4", "\"columns\": 9")).is_err());
    }

    #[test]
    fn parse_theme_should_reject_the_reserved_id() {
        assert!(parse_theme(&VALID.replace("\"vidrio\"", "\"default\"")).is_err());
    }

    #[test]
    fn parse_theme_should_reject_image_background_without_image() {
        let text = VALID.replace(
            r##"{ "type": "gradient", "colors": ["#1B2A4A", "#0A0D14"], "angle": 180 }"##,
            r#"{ "type": "image" }"#,
        );
        assert!(parse_theme(&text).is_err());
    }

    #[test]
    fn bundled_themes_should_be_valid() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("themes");
        let problems: Vec<String> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let text = std::fs::read_to_string(entry.path().join(THEME_FILE)).ok()?;
                match parse_theme(&text) {
                    Ok(theme) if entry.file_name().to_str() == Some(theme.id.as_str()) => None,
                    Ok(theme) => Some(format!("{}: folder differs from id", theme.id)),
                    Err(problem) => Some(format!("{}: {problem}", entry.path().display())),
                }
            })
            .collect();
        assert_eq!(problems, Vec::<String>::new());
    }

    #[test]
    fn is_theme_id_should_reject_path_tricks() {
        assert!(!is_theme_id("../evil"));
    }

    #[test]
    fn is_asset_name_should_accept_images() {
        assert!(is_asset_name("background.webp"));
    }

    #[test]
    fn is_asset_name_should_reject_other_files() {
        assert!(!is_asset_name("payload.sh"));
    }

    #[test]
    fn is_asset_name_should_reject_paths() {
        assert!(!is_asset_name("../background.png"));
    }

    #[test]
    fn assets_should_list_the_background_image() {
        let text = VALID.replace(
            r##"{ "type": "gradient", "colors": ["#1B2A4A", "#0A0D14"], "angle": 180 }"##,
            r#"{ "type": "image", "image": "bg.jpg" }"#,
        );
        assert_eq!(
            parse_theme(&text).map(|theme| theme.assets().join(",")),
            Ok("bg.jpg".to_owned())
        );
    }
}
