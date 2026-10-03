//! Module-owned package boundaries from logical names, before any upload.
//! This is intake policy, not a parser or a claim that the dump is valid.
use collector_core::PackageFileName;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct DiscoveredImportPackage {
    pub label: String,
    pub file_indices: Vec<usize>,
}

pub fn discover(names: &[PackageFileName], cartridge_files: bool) -> Vec<DiscoveredImportPackage> {
    let lower: Vec<_> = names
        .iter()
        .map(|name| name.as_str().to_ascii_lowercase())
        .collect();
    let by_name: BTreeMap<_, _> = lower
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let mut assigned = BTreeSet::new();
    let mut packages = Vec::new();
    // A redumper TOC owns its same-stem sidecars. Multiple dumps may share a folder.
    let mut anchors: Vec<_> = lower
        .iter()
        .enumerate()
        .filter(|(_, name)| name.ends_with(".toc"))
        .collect();
    // More specific stems own their files before a shorter prefix can claim them.
    anchors.sort_by_key(|(_, name)| std::cmp::Reverse(name.len()));
    for (index, name) in anchors {
        let stem = &name[..name.len() - 4];
        let prefix = format!("{stem}.");
        let mut file_indices: Vec<_> = by_name
            .range(prefix.as_str()..)
            .take_while(|(n, _)| n.starts_with(&prefix))
            .map(|(_, i)| *i)
            .filter(|i| !assigned.contains(i))
            .collect();
        if !assigned.contains(&index) && !file_indices.contains(&index) {
            file_indices.push(index);
        }
        file_indices.sort_unstable();
        if file_indices.is_empty() {
            continue;
        }
        assigned.extend(file_indices.iter().copied());
        let basename = names[index].as_str().rsplit('/').next().unwrap();
        packages.push(DiscoveredImportPackage {
            label: basename[..basename.len() - 4].to_owned(),
            file_indices,
        });
    }
    // A CUE may refer to arbitrarily named files or nested paths. Preserve that
    // directory tree together rather than guessing track ownership. Multiple CUEs
    // in one directory remain one ambiguous package for the normal analyzer.
    let mut cue_dirs = BTreeMap::new();
    for (index, name) in lower
        .iter()
        .enumerate()
        .filter(|(i, name)| !assigned.contains(i) && name.ends_with(".cue"))
    {
        let directory = name.rsplit_once('/').map_or("", |(dir, _)| dir);
        cue_dirs.entry(directory.to_owned()).or_insert(index);
    }
    // An independently anchored child directory is a separate package.
    let mut cue_dirs: Vec<_> = cue_dirs.into_iter().collect();
    cue_dirs.sort_by_key(|(directory, _)| std::cmp::Reverse(directory.len()));
    for (directory, index) in cue_dirs {
        let prefix = format!("{directory}/");
        let mut file_indices: Vec<_> = if directory.is_empty() {
            (0..names.len()).filter(|i| !assigned.contains(i)).collect()
        } else {
            by_name
                .range(prefix.as_str()..)
                .take_while(|(n, _)| n.starts_with(&prefix))
                .map(|(_, i)| *i)
                .filter(|i| !assigned.contains(i))
                .collect()
        };
        file_indices.sort_unstable();
        if file_indices.is_empty() {
            continue;
        }
        assigned.extend(file_indices.iter().copied());
        let label = names[index]
            .as_str()
            .rsplit_once('/')
            .map_or("CD package", |(dir, _)| dir.rsplit('/').next().unwrap());
        packages.push(DiscoveredImportPackage {
            label: label.into(),
            file_indices,
        });
    }
    if cartridge_files {
        for (index, name) in lower
            .iter()
            .enumerate()
            .filter(|(i, _)| !assigned.contains(i))
        {
            if name.rsplit('.').next().is_some_and(|ext| {
                matches!(
                    ext,
                    "nes" | "n64" | "z64" | "v64" | "sfc" | "smc" | "pce" | "iso"
                )
            }) {
                packages.push(DiscoveredImportPackage {
                    label: names[index].as_str().rsplit('/').next().unwrap().into(),
                    file_indices: vec![index],
                });
            }
        }
    }
    packages.sort_by_key(|package| package.file_indices[0]);
    packages
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_and_same_folder_dumps_have_disjoint_packages_and_keep_sidecars() {
        let names = [
            "Dumps/a/disc.toc",
            "Dumps/a/disc.scram",
            "Dumps/a/disc.state",
            "Dumps/a/disc.subcode",
            "Dumps/a/second.TOC",
            "Dumps/a/second.scram",
            "Dumps/b/disc.cue",
            "Dumps/b/tracks/track.bin",
            "Dumps/readme.txt",
            "Dumps/game.nes",
        ]
        .map(|name| PackageFileName::new(name).unwrap());
        let music = discover(&names, false);
        assert_eq!(
            music
                .iter()
                .map(|p| p.file_indices.clone())
                .collect::<Vec<_>>(),
            vec![vec![0, 1, 2, 3], vec![4, 5], vec![6, 7]]
        );
        assert_eq!(discover(&names, true).last().unwrap().file_indices, vec![9]);
    }
    #[test]
    fn overlapping_stems_and_nested_cue_directories_remain_separate() {
        let names = [
            "disc.toc",
            "disc.scram",
            "disc.extra.toc",
            "disc.extra.scram",
            "other/a.cue",
            "other/a.bin",
            "other/child/b.cue",
            "other/child/b.bin",
        ]
        .map(|name| PackageFileName::new(name).unwrap());
        assert_eq!(
            discover(&names, false)
                .iter()
                .map(|p| p.file_indices.clone())
                .collect::<Vec<_>>(),
            vec![vec![0, 1], vec![2, 3], vec![4, 5], vec![6, 7]]
        );
    }
    #[test]
    fn cues_with_uncertain_ownership_stay_together_and_missing_companions_are_not_invented() {
        let names = [
            "Folder/a.cue",
            "Folder/b.cue",
            "Folder/one.bin",
            "Folder/nested/two.bin",
        ]
        .map(|name| PackageFileName::new(name).unwrap());
        assert_eq!(discover(&names, false)[0].file_indices, vec![0, 1, 2, 3]);
        let missing = [PackageFileName::new("Folder/disc.toc").unwrap()];
        assert_eq!(discover(&missing, false)[0].file_indices, vec![0]);
    }
}
