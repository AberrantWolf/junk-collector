//! Music-owned frozen catalog contracts shared with provider adapters.
use collector_core::{Error, Result};
use serde::{Deserialize, Serialize};
pub const DBAR_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DbarQuery {
    pub track_count: u8,
    pub accuraterip_id1: u32,
    pub accuraterip_id2: u32,
    pub cddb_id: u32,
}

impl DbarQuery {
    pub fn validate(&self) -> Result<()> {
        if !(1..=99).contains(&self.track_count) {
            return Err(Error::Invalid(
                "AccurateRip query requires between 1 and 99 tracks".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DbarOutcome {
    Data,
    NoData,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DbarSnapshotObservation {
    pub schema_version: u32,
    pub query: DbarQuery,
    pub outcome: DbarOutcome,
    pub response_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParsedDbarTrack {
    pub confidence: u8,
    pub checksum: u32,
    pub checksum_450: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParsedDbarResponse {
    pub track_count: u8,
    pub accuraterip_id1: u32,
    pub accuraterip_id2: u32,
    pub cddb_id: u32,
    pub tracks: Vec<ParsedDbarTrack>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParsedDbarSnapshot {
    pub responses: Vec<ParsedDbarResponse>,
}

impl DbarSnapshotObservation {
    pub fn validate(&self) -> Result<()> {
        self.query.validate()?;
        if self.schema_version != DBAR_SNAPSHOT_SCHEMA_VERSION
            || (self.outcome == DbarOutcome::NoData && self.response_count != 0)
            || (self.outcome == DbarOutcome::Data && self.response_count == 0)
        {
            return Err(Error::Invalid(
                "invalid AccurateRip snapshot observation".into(),
            ));
        }
        Ok(())
    }
}
