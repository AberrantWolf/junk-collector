//! Standard-system identities and catalog policy; no filename-based identity.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StandardGameSystem {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub byte_domain: String,
}
pub fn standard_game_systems() -> Vec<StandardGameSystem> {
    [
        (
            "nes",
            "Nintendo - Nintendo Entertainment System",
            &["NES", "Famicom"][..],
            "org.junk.game.nes.ines-payload",
        ),
        (
            "n64",
            "Nintendo - Nintendo 64",
            &["N64", "Nintendo 64"][..],
            "org.junk.game.n64.z64-big-endian",
        ),
        (
            "snes",
            "Nintendo - Super Nintendo Entertainment System",
            &["SNES", "Super Famicom", "Super Nintendo"][..],
            "org.junk.game.snes.headerless-payload",
        ),
        (
            "pce",
            "NEC - PC Engine - TurboGrafx-16",
            &["PCE", "PC Engine", "TurboGrafx-16"][..],
            "org.junk.game.pce.hucard-payload",
        ),
        (
            "ps1",
            "Sony - PlayStation",
            &["PS1", "PSX", "Sony PlayStation", "PlayStation"][..],
            "org.junk.game.cd.raw-2352-track",
        ),
        (
            "saturn",
            "Sega - Saturn",
            &["Saturn", "Sega Saturn"][..],
            "org.junk.game.cd.raw-2352-track",
        ),
        (
            "segacd",
            "Sega - Mega CD & Sega CD",
            &["Sega CD", "Mega CD"][..],
            "org.junk.game.cd.raw-2352-track",
        ),
        (
            "megadrive",
            "Sega - Mega Drive - Genesis",
            &["Mega Drive", "Megadrive", "Genesis", "Sega Genesis"][..],
            "org.junk.game.raw-bytes",
        ),
        (
            "gb",
            "Nintendo - Game Boy",
            &["Game Boy", "GB"][..],
            "org.junk.game.raw-bytes",
        ),
        (
            "gbc",
            "Nintendo - Game Boy Color",
            &["Game Boy Color", "GBC"][..],
            "org.junk.game.raw-bytes",
        ),
        (
            "gba",
            "Nintendo - Game Boy Advance",
            &["Game Boy Advance", "GBA"][..],
            "org.junk.game.raw-bytes",
        ),
        (
            "sms",
            "Sega - Master System - Mark III",
            &["Master System", "SMS"][..],
            "org.junk.game.raw-bytes",
        ),
        (
            "gg",
            "Sega - Game Gear",
            &["Game Gear", "GG"][..],
            "org.junk.game.raw-bytes",
        ),
    ]
    .into_iter()
    .map(|(id, name, aliases, domain)| StandardGameSystem {
        id: id.into(),
        name: name.into(),
        aliases: aliases.iter().map(|s| (*s).into()).collect(),
        byte_domain: domain.into(),
    })
    .collect()
}
fn key(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
pub fn standard_system(value: &str) -> Option<StandardGameSystem> {
    let value = key(value);
    standard_game_systems().into_iter().find(|system| {
        key(&system.id) == value
            || key(&system.name) == value
            || system.aliases.iter().any(|alias| key(alias) == value)
    })
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GameDetectionStatus {
    Recognized,
    Ambiguous,
    Unrecognized,
    Malformed,
    Unsupported,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CatalogDetection {
    pub status: GameDetectionStatus,
    pub dialect: String,
    pub system: Option<StandardGameSystem>,
    pub system_choices: Vec<StandardGameSystem>,
    pub catalog: Option<String>,
    pub version: String,
    pub byte_domains: Vec<String>,
    pub reasons: Vec<String>,
}
pub fn detect_standard_catalog(
    dialect: &str,
    name: &str,
    description: &str,
    author: &str,
    url: &str,
    version: &str,
) -> CatalogDetection {
    let text = format!("{name} {description} {author} {url}").to_ascii_lowercase();
    let no_intro =
        text.contains("no-intro") || text.contains("no intro") || text.contains("nointro");
    let family = if no_intro && text.contains("redump") {
        None
    } else if no_intro {
        Some("No-Intro")
    } else if text.contains("redump") {
        Some("Redump")
    } else {
        None
    };
    let strip = |value: &str| value.split('(').next().unwrap_or(value).trim().to_owned();
    let a = standard_system(&strip(name));
    let b = standard_system(&strip(description));
    let conflict = a
        .as_ref()
        .zip(b.as_ref())
        .is_some_and(|(a, b)| a.id != b.id);
    let system_choices = if conflict {
        a.iter().chain(b.iter()).cloned().collect()
    } else {
        vec![]
    };
    let system = a.or(b);
    let mut result = CatalogDetection {
        status: GameDetectionStatus::Recognized,
        dialect: dialect.into(),
        system,
        system_choices,
        catalog: family.map(str::to_owned),
        version: version.into(),
        byte_domains: vec![],
        reasons: vec![],
    };
    if conflict {
        result.status = GameDetectionStatus::Ambiguous;
        result
            .reasons
            .push("Catalog name and description identify different standard systems".into());
        result.system = None;
    } else if result.system.is_none() || family.is_none() || version.trim().is_empty() {
        result.status = GameDetectionStatus::Unsupported;
        result.reasons.push("A recognized standard system, No-Intro/Redump convention and catalog version are required".into());
    } else if result.system.as_ref().is_some_and(|s| s.id == "n64")
        && [
            "byteswapped",
            "byte-swapped",
            "littleendian",
            "little-endian",
        ]
        .iter()
        .any(|hint| text.contains(hint))
    {
        result.status = GameDetectionStatus::Unsupported;
        result
            .reasons
            .push("Only big-endian N64 catalog checksums are supported".into());
    } else if let Some(system) = &result.system {
        if (family == Some("Redump")) != (system.byte_domain == "org.junk.game.cd.raw-2352-track") {
            result.status = GameDetectionStatus::Unsupported;
            result.reasons.push(
                "This system/catalog convention has no qualified automatic byte interpretation"
                    .into(),
            );
        } else {
            result.byte_domains.push(
                if system.id == "nes" && text.contains("headered") && !text.contains("headerless") {
                    "org.junk.game.raw-bytes".into()
                } else {
                    system.byte_domain.clone()
                },
            );
            result
                .reasons
                .push("System and catalog convention recognized from document header".into());
        }
    }
    result
}

/// Only document-supported conflicting systems can be explicitly corrected.
pub fn corrected_catalog(detection: &CatalogDetection, system: &str) -> Option<CatalogDetection> {
    if !matches!(detection.status, GameDetectionStatus::Ambiguous)
        || detection.version.trim().is_empty()
    {
        return None;
    }
    let choice = detection
        .system_choices
        .iter()
        .find(|s| s.name == system)?
        .clone();
    let family = detection.catalog.as_deref()?;
    if (family == "Redump") != (choice.byte_domain == "org.junk.game.cd.raw-2352-track") {
        return None;
    }
    let mut result = detection.clone();
    result.system = Some(choice.clone());
    result.status = GameDetectionStatus::Recognized;
    result.byte_domains = vec![choice.byte_domain];
    result
        .reasons
        .push("Explicit correction selected from conflicting document systems".into());
    Some(result)
}
