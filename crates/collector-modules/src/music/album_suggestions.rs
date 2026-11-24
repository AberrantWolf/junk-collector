//! Suggest which import packages are discs of one album, from package names
//! only. No media is read and nothing is grouped without the curator.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Packages whose names differ only by a disc number.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AlbumGroupSuggestion {
    /// Stable grouping key derived from the shared name.
    pub key: String,
    /// The shared name, as written in the first package.
    pub title: String,
    pub members: Vec<AlbumGroupSuggestionMember>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AlbumGroupSuggestionMember {
    /// Index into the names that were passed in.
    pub index: u32,
    pub disc: u16,
}

/// Group names like "Album (Disc 1)" and "Album (Disc 2)", "Album CD1" and
/// "Album CD2", or "Album/Disc 1" and "Album/Disc 2". A group needs at least
/// two packages with different disc numbers; names that repeat a disc number
/// are left alone, since they may be separate copies.
pub fn suggest_album_groups(names: &[&str]) -> Vec<AlbumGroupSuggestion> {
    let mut groups: BTreeMap<String, (String, Vec<AlbumGroupSuggestionMember>)> = BTreeMap::new();
    for (index, name) in names.iter().enumerate() {
        let Some((base, disc)) = split_disc(name) else {
            continue;
        };
        let key = normalize(&base);
        if key.is_empty() {
            continue;
        }
        groups
            .entry(key)
            .or_insert_with(|| (base.clone(), Vec::new()))
            .1
            .push(AlbumGroupSuggestionMember {
                index: index as u32,
                disc,
            });
    }
    groups
        .into_iter()
        .filter_map(|(key, (title, mut members))| {
            members.sort_by_key(|member| (member.disc, member.index));
            let distinct = members.windows(2).all(|pair| pair[0].disc != pair[1].disc);
            (members.len() >= 2 && distinct).then_some(AlbumGroupSuggestion {
                key,
                title,
                members,
            })
        })
        .collect()
}

/// Split a trailing disc marker off a package name.
fn split_disc(name: &str) -> Option<(String, u16)> {
    let name = name.trim_end_matches('/');
    // "Album/Disc 1": the folder is the disc, its parent the album.
    if let Some((parent, last)) = name.rsplit_once('/')
        && let Some(disc) = super::discovery::disc_number(last.trim())
    {
        let base = clean_base(parent.rsplit('/').next().unwrap_or(parent));
        return (!base.is_empty()).then_some((base, disc));
    }
    let last = name.rsplit('/').next().unwrap_or(name);
    let lower = last.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut end = lower
        .trim_end_matches(|c: char| c.is_whitespace() || matches!(c, ')' | ']' | '}'))
        .len();
    // "Disc 1 of 2": drop the total.
    let digits_start = |end: usize| {
        let mut start = end;
        while start > 0 && bytes[start - 1].is_ascii_digit() {
            start -= 1;
        }
        start
    };
    let total = digits_start(end);
    if total < end {
        let before = lower[..total].trim_end();
        if let Some(rest) = before.strip_suffix("of")
            && rest.ends_with(char::is_whitespace)
        {
            end = rest.trim_end().len();
        }
    }
    let start = digits_start(end);
    if start == end || end - start > 3 {
        return None;
    }
    let disc: u16 = lower[start..end].parse().ok().filter(|value| *value > 0)?;
    let marker_end = lower[..start]
        .trim_end_matches([' ', '_', '-', '.', '#'])
        .len();
    let head = &lower[..marker_end];
    let marker = ["disc", "disk", "cd"]
        .into_iter()
        .find(|marker| head.ends_with(marker))?;
    let marker_start = marker_end - marker.len();
    if marker_start > 0 && bytes[marker_start - 1].is_ascii_alphanumeric() {
        return None;
    }
    let base = clean_base(&last[..marker_start]);
    (!base.is_empty()).then_some((base, disc))
}

fn clean_base(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(|c: char| {
            c.is_whitespace() || matches!(c, '-' | '_' | '(' | '[' | '{' | '.' | ',')
        })
        .trim()
        .to_string()
}

/// Case and punctuation do not separate a group.
fn normalize(value: &str) -> String {
    value
        .split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups(names: &[&str]) -> Vec<(String, Vec<(u32, u16)>)> {
        suggest_album_groups(names)
            .into_iter()
            .map(|g| {
                (
                    g.title,
                    g.members.into_iter().map(|m| (m.index, m.disc)).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn common_disc_spellings_group_by_their_shared_name() {
        assert_eq!(
            groups(&[
                "Harbour Lights (Disc 2)",
                "Harbour Lights (Disc 1)",
                "Other"
            ]),
            vec![("Harbour Lights".into(), vec![(1, 1), (0, 2)])]
        );
        assert_eq!(
            groups(&[
                "Night Songs CD1",
                "night songs - cd2",
                "Night Songs [Disc 3 of 3]"
            ]),
            vec![("Night Songs".into(), vec![(0, 1), (1, 2), (2, 3)])]
        );
        assert_eq!(
            groups(&["Live/Disc 1", "Live/Disc 2"]),
            vec![("Live".into(), vec![(0, 1), (1, 2)])]
        );
        assert_eq!(
            groups(&["Box_Disk_01", "Box_Disk_02"]),
            vec![("Box".into(), vec![(0, 1), (1, 2)])]
        );
    }

    #[test]
    fn lone_discs_repeats_and_lookalikes_are_not_grouped() {
        assert!(groups(&["Album (Disc 1)", "Another (Disc 2)"]).is_empty());
        // Two "Disc 1" folders may be two copies; leave them to the curator.
        assert!(groups(&["Album (Disc 1)", "Album Disc 1"]).is_empty());
        // "ABCD" only ends in letters that spell a marker.
        assert!(groups(&["ABCD 1", "ABCD 2"]).is_empty());
        assert!(groups(&["Disc 1", "Disc 2"]).is_empty());
        assert!(groups(&["Album 1999", "Album 2000"]).is_empty());
    }
}
