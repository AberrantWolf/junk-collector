//! Music intake boundaries from logical paths only; no media reads or tag guesses.
use crate::import_discovery::DiscoveredImportPackage;
use collector_core::PackageFileName;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn disc_number(name: &str) -> Option<u16> {
    let lower = name.to_ascii_lowercase();
    lower
        .strip_prefix("disc")
        .or_else(|| lower.strip_prefix("cd"))
        .map(|suffix| suffix.trim_start_matches([' ', '_', '-']))
        .filter(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
        .and_then(|suffix| suffix.parse().ok())
        .filter(|number| *number > 0)
}

pub(super) fn discover(names: &[PackageFileName]) -> Vec<DiscoveredImportPackage> {
    // Raw-disc anchors retain their established package ownership and analysis.
    let mut packages = crate::import_discovery::discover(names, false);
    let mut used = packages
        .iter()
        .flat_map(|p| p.file_indices.iter().copied())
        .collect::<BTreeSet<_>>();
    let mut groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, name) in names.iter().enumerate() {
        if used.contains(&index) || !super::files::is_audio_name(name.as_str()) {
            continue;
        }
        let root = match name.as_str().split_once('/') {
            Some((first, _)) => first,
            None => name.as_str(),
        };
        groups.entry(root).or_default().push(index);
        used.insert(index);
    }
    for (index, name) in names.iter().enumerate() {
        if used.contains(&index) {
            continue;
        }
        if let Some((root, _)) = name.as_str().split_once('/')
            && let Some(files) = groups.get_mut(root)
        {
            files.push(index);
        }
    }
    for (root, mut file_indices) in groups {
        file_indices.sort_unstable();
        packages.push(DiscoveredImportPackage {
            label: root.into(),
            file_indices,
        });
    }
    packages.sort_by_key(|package| package.file_indices[0]);
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
    fn top_level_albums_keep_all_formats_discs_and_named_sections() {
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
                "Album/Main OST/01.mp3",
                "Other Album/01.wav",
            ];
            let packages = discovered(&paths);
            assert_eq!(packages.len(), 2);
            assert_eq!(packages["Album"].len(), 8);
            assert_eq!(packages["Other Album"], ["Other Album/01.wav"]);
            assert_eq!(
                discovered(&paths.into_iter().rev().collect::<Vec<_>>()),
                packages
            );
        }
    }

    #[test]
    fn loose_formats_and_decorated_subdirectories_stay_with_the_top_level_item() {
        let packages = discovered(&[
            "Album/01.mp3",
            "Album/01.flac",
            "Album/cover.jpg",
            "Album/Bonus/02.mp3",
            "Album/FLAC 24-bit/01.flac",
            "Separate disc/CD1/01.wav",
        ]);
        assert_eq!(packages.len(), 2);
        assert_eq!(packages["Album"].len(), 5);
        assert_eq!(packages["Separate disc"], ["Separate disc/CD1/01.wav"]);
        // Direct loose files in an import volume are individual media items.
        assert_eq!(discovered(&["a.flac", "b.mp3"]).len(), 2);
    }

    #[test]
    fn top_level_format_and_disc_names_are_independent_items() {
        let packages = discovered(&["FLAC/CD1/01.flac", "FLAC/CD2/01.flac", "WAV/CD1/01.wav"]);
        assert_eq!(packages["FLAC"].len(), 2);
        assert_eq!(packages["WAV"].len(), 1);
        assert_eq!(discovered(&["Disc 1/01.flac", "Disc 2/01.mp3"]).len(), 2);
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
