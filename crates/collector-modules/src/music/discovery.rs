//! Music intake boundaries from logical paths only; no media reads or tag guesses.
use crate::import_discovery::DiscoveredImportPackage;
use collector_core::PackageFileName;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct Group<'a> {
    format_scopes: BTreeSet<&'a str>,
    files: Vec<usize>,
}

fn parent(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

fn disc(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower
        .strip_prefix("disc")
        .or_else(|| lower.strip_prefix("cd"))
        .map(|suffix| suffix.trim_start_matches([' ', '_', '-']))
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
}

fn format(name: &str) -> Option<&'static str> {
    match name.to_ascii_lowercase().as_str() {
        "flac" => Some("FLAC"),
        "mp3" => Some("MP3"),
        "wav" => Some("WAV"),
        _ => None,
    }
}

fn contains(directory: &str, path: &str) -> bool {
    directory.is_empty()
        || path
            .strip_prefix(directory)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

pub(super) fn discover(names: &[PackageFileName]) -> Vec<DiscoveredImportPackage> {
    let mut packages = crate::import_discovery::discover(names, false);
    let mut used = packages
        .iter()
        .flat_map(|package| package.file_indices.iter().copied())
        .collect::<BTreeSet<_>>();
    // The album root and explicit encoding are independent: both FLAC/Disc 1
    // and Disc 1/FLAC belong to the same FLAC version, never to its MP3 sibling.
    let mut groups: BTreeMap<(&str, Option<&str>), Group<'_>> = BTreeMap::new();
    for (index, name) in names.iter().enumerate() {
        if used.contains(&index) || !super::files::is_audio_name(name.as_str()) {
            continue;
        }
        let mut root = parent(name.as_str()).0;
        let mut encoding = None;
        let mut scope = None;
        let mut saw_disc = false;
        while !root.is_empty() {
            let (up, leaf) = parent(root);
            if !saw_disc && disc(leaf) {
                saw_disc = true;
            } else if encoding.is_none() && format(leaf).is_some() {
                encoding = format(leaf);
                scope = Some(root);
            } else {
                break;
            }
            root = up;
        }
        let group = groups.entry((root, encoding)).or_default();
        group.files.push(index);
        group.format_scopes.extend(scope);
        used.insert(index);
    }
    // Assign companions to the deepest independent album. A companion within
    // an encoding folder stays with that encoding. Shared parent companions
    // belong once to the first encoding in stable key order (FLAC before MP3),
    // independent of file enumeration; the intake contract forbids overlap.
    for (index, name) in names.iter().enumerate() {
        if used.contains(&index) {
            continue;
        }
        let owner = groups
            .iter()
            .filter(|((root, _), _)| contains(root, name.as_str()))
            .max_by_key(|((root, encoding), group)| {
                (
                    root.len(),
                    group
                        .format_scopes
                        .iter()
                        .filter(|scope| contains(scope, name.as_str()))
                        .map(|scope| scope.len())
                        .max()
                        .unwrap_or(0),
                    // max_by_key otherwise chooses the *last* equal key.
                    std::cmp::Reverse(*encoding),
                )
            })
            .map(|(key, _)| *key);
        if let Some(owner) = owner {
            groups.get_mut(&owner).unwrap().files.push(index);
        }
    }
    for ((root, encoding), group) in &groups {
        let album = if root.is_empty() {
            encoding.unwrap_or("Digital purchase")
        } else {
            parent(root).1
        };
        let alternatives = groups.keys().filter(|(other, _)| other == root).count() > 1;
        let label = match encoding {
            Some(encoding) if alternatives && !root.is_empty() => format!("{album} ({encoding})"),
            _ => album.to_owned(),
        };
        let mut file_indices = group.files.clone();
        file_indices.sort_unstable();
        packages.push(DiscoveredImportPackage {
            label,
            file_indices,
        });
    }
    packages
}

#[cfg(test)]
mod tests {
    use super::*;

    fn discovered(paths: &[&str]) -> BTreeMap<String, Vec<String>> {
        let names = paths
            .iter()
            .map(|path| PackageFileName::new(*path).unwrap())
            .collect::<Vec<_>>();
        let packages = discover(&names);
        let mut assigned = BTreeSet::new();
        packages
            .into_iter()
            .map(|package| {
                let mut files = package
                    .file_indices
                    .into_iter()
                    .map(|index| {
                        assert!(assigned.insert(index), "overlapping package membership");
                        paths[index].to_owned()
                    })
                    .collect::<Vec<_>>();
                files.sort();
                (package.label, files)
            })
            .collect()
    }

    #[test]
    fn format_wrapper_uses_album_and_retains_parent_companions() {
        let paths = [
            "Game Name OST/FLAC/01.flac",
            "Game Name OST/FLAC/02.flac",
            "Game Name OST/cover.png",
            "Game Name OST/Booklet/pages/booklet.pdf",
            "Game Name OST/receipt.txt",
            "Other Album/FLAC/01.flac",
            "Other Album/cover.jpg",
        ];
        let packages = discovered(&paths);
        assert_eq!(packages["Game Name OST"].len(), 5);
        assert!(
            packages["Game Name OST"]
                .iter()
                .all(|p| p.starts_with("Game Name OST/"))
        );
        assert_eq!(packages["Other Album"].len(), 2);
        assert_eq!(packages.len(), 2);
    }

    #[test]
    fn alternate_encodings_and_both_disc_layouts_stay_distinct() {
        for (first, second) in [
            ("Album/flac/Disc 1/01.flac", "Album/flac/CD_2/01.flac"),
            ("Album/Disc 1/flac/01.flac", "Album/CD_2/FLAC/01.flac"),
        ] {
            let paths = [
                first,
                second,
                "Album/MP3/Disc 1/01.mp3",
                "Album/MP3/cover.jpg",
                "Album/cover.png",
                "Album/Booklet/booklet.pdf",
                "Album/DLC OST/WAV/01.wav",
                "Album/DLC OST/cover.jpg",
                "Album/Main OST/01.mp3",
                "Album/Main OST/booklet.pdf",
            ];
            let packages = discovered(&paths);
            assert_eq!(packages.len(), 4);
            let flac = &packages["Album (FLAC)"];
            assert_eq!(flac.len(), 4);
            assert!(flac.contains(&first.into()) && flac.contains(&second.into()));
            assert!(flac.contains(&"Album/cover.png".into()));
            assert!(flac.contains(&"Album/Booklet/booklet.pdf".into()));
            assert_eq!(packages["Album (MP3)"].len(), 2);
            assert_eq!(packages["DLC OST"].len(), 2);
            assert_eq!(packages["Main OST"].len(), 2);
            let reversed = paths.into_iter().rev().collect::<Vec<_>>();
            assert_eq!(
                discovered(&reversed),
                packages,
                "ownership must not depend on enumeration"
            );
        }
    }

    #[test]
    fn direct_tracks_and_named_folders_are_not_encoding_aliases() {
        let packages = discovered(&[
            "Album/01.mp3",
            "Album/FLAC/01.flac",
            "Album/cover.jpg",
            "Album/Bonus/01.mp3",
            "Album/Bonus/cover.jpg",
            "Album/FLAC Extras/01.flac",
            "Album/FLAC Extras/cover.jpg",
            "FLAC/Separate Album/01.flac",
        ]);
        assert_eq!(packages.len(), 5);
        assert_eq!(packages["Album"].len(), 2);
        assert_eq!(packages["Album (FLAC)"], ["Album/FLAC/01.flac"]);
        assert_eq!(packages["Bonus"].len(), 2);
        assert_eq!(packages["FLAC Extras"].len(), 2);
        assert_eq!(packages["Separate Album"], ["FLAC/Separate Album/01.flac"]);
    }

    #[test]
    fn root_format_and_disc_folders_work_without_inventing_album_names() {
        let packages = discovered(&["FLAC/CD1/01.flac", "FLAC/CD2/01.flac", "cover.jpg"]);
        assert_eq!(packages["FLAC"].len(), 3);
        let packages = discovered(&["Disc 1/01.flac", "Disc 2/01.mp3", "cover.jpg"]);
        assert_eq!(packages["Digital purchase"].len(), 3);
    }

    #[test]
    fn raw_disc_packages_keep_their_anchored_members() {
        let paths = [
            "Album/FLAC/01.flac",
            "Album/cover.jpg",
            "Album/Raw/disc.cue",
            "Album/Raw/disc.bin",
            "Album/Raw/cover.png",
        ];
        let packages = discovered(&paths);
        assert_eq!(packages["Raw"].len(), 3);
        assert_eq!(packages["Album"].len(), 2);
    }
}
