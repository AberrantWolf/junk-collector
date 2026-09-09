//! Domain-owned schemas and capability registration. No application flow or I/O.
use collector_core::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub available: bool,
    pub limitation: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleDescriptor {
    pub id: ModuleId,
    pub schema_version: u32,
    pub display_name: String,
    pub capabilities: Vec<Capability>,
}
pub trait CollectionModule: Send + Sync {
    fn descriptor(&self) -> ModuleDescriptor;
    fn validate(&self, data: &ModuleData) -> Result<()>;
}
#[derive(Default)]
pub struct Registry {
    modules: BTreeMap<String, Box<dyn CollectionModule>>,
}
impl Registry {
    pub fn builtins() -> Self {
        let mut result = Self::default();
        result.register(Box::new(Games)).unwrap();
        result.register(Box::new(Music)).unwrap();
        result
    }
    pub fn register(&mut self, module: Box<dyn CollectionModule>) -> Result<()> {
        let id = module.descriptor().id;
        id.validate()?;
        if self.modules.contains_key(&id.0) {
            return Err(Error::Invalid("duplicate module".into()));
        }
        self.modules.insert(id.0, module);
        Ok(())
    }
    pub fn validate(&self, data: &ModuleData) -> Result<()> {
        match self.modules.get(&data.module.0) {
            Some(m) => m.validate(data),
            None => data.module.validate(),
        }
    }
    pub fn descriptors(&self) -> Vec<ModuleDescriptor> {
        self.modules.values().map(|m| m.descriptor()).collect()
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameEdition {
    pub platform: Option<String>,
    pub region: Option<String>,
    pub publisher: Option<String>,
    pub release_year: Option<u16>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicEdition {
    pub artist: Option<String>,
    pub label: Option<String>,
    pub release_year: Option<u16>,
    #[serde(default)]
    pub tracks: Vec<Track>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub position: u16,
    pub title: String,
    pub start: SampleFrame,
    pub length: SampleFrame,
}
pub struct Games;
pub struct Music;
fn descriptor(id: &str, name: &str) -> ModuleDescriptor {
    ModuleDescriptor {
        id: ModuleId(id.into()),
        schema_version: 1,
        display_name: name.into(),
        capabilities: vec![
            Capability {
                name: "inventory".into(),
                available: true,
                limitation: None,
            },
            Capability {
                name: "preservation".into(),
                available: true,
                limitation: None,
            },
            Capability {
                name: "catalog_verification".into(),
                available: false,
                limitation: Some(
                    "Analyzer/provider migration pending; integrity verification is separate."
                        .into(),
                ),
            },
        ],
    }
}
impl CollectionModule for Games {
    fn descriptor(&self) -> ModuleDescriptor {
        descriptor("org.junk.games", "Video games")
    }
    fn validate(&self, data: &ModuleData) -> Result<()> {
        if data.schema_version != 1 {
            return Ok(());
        } // preserve future schemas without rewriting their payload
        serde_json::from_value::<GameEdition>(data.payload.clone())
            .map_err(|e| Error::Invalid(e.to_string()))?;
        Ok(())
    }
}
impl CollectionModule for Music {
    fn descriptor(&self) -> ModuleDescriptor {
        descriptor("org.junk.music", "Music CDs")
    }
    fn validate(&self, data: &ModuleData) -> Result<()> {
        if data.schema_version != 1 {
            return Ok(());
        }
        let music: MusicEdition = serde_json::from_value(data.payload.clone())
            .map_err(|e| Error::Invalid(e.to_string()))?;
        for track in music.tracks {
            track
                .start
                .0
                .checked_add(track.length.0)
                .ok_or_else(|| Error::Invalid("track span overflow".into()))?;
        }
        Ok(())
    }
}
/// Deterministic best guess. Identity strength precedes preferred language; manual choice survives refresh.
pub fn resolve_metadata(
    item: &mut Item,
    candidates: Vec<MetadataCandidate>,
    preferred_language: &str,
) {
    let manual = item
        .metadata
        .selected
        .as_ref()
        .and_then(|id| candidates.iter().find(|c| &c.id == id));
    let selected = manual.or_else(|| {
        candidates.iter().max_by(|a, b| {
            (a.strength, a.language == preferred_language)
                .cmp(&(b.strength, b.language == preferred_language))
                .then_with(|| b.id.cmp(&a.id))
        })
    });
    if let Some(selected) = selected {
        if !item.metadata.overrides.contains_key("title") {
            item.title = selected.title.clone();
        }
        if !item.metadata.overrides.contains_key("subtitle") {
            item.subtitle = selected.subtitle.clone();
        }
        if manual.is_none() && item.identity != IdentityState::Confirmed {
            item.identity = IdentityState::Tentative;
        }
    }
    item.metadata.candidates = candidates;
}
