use collector_app::*;
use collector_core::*;
use collector_providers::StandardCatalogInspector;
use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
    sync::Mutex,
};
#[derive(Default)]
struct Staging(Mutex<BTreeMap<String, Vec<u8>>>);
impl UploadedCatalogPort for Staging {
    fn stage_uploaded_catalog(&self, bytes: &[u8]) -> Result<Digest> {
        let digest = Digest::sha256(bytes);
        self.0
            .lock()
            .unwrap()
            .insert(digest.hex.clone(), bytes.to_vec());
        Ok(digest)
    }
    fn uploaded_catalog(&self, upload: &UploadedDatCatalog) -> Result<Box<dyn MediaReader>> {
        Ok(Box::new(Cursor::new(
            self.0.lock().unwrap()[&upload.digest.hex].clone(),
        )))
    }
}
fn dat(system: &str, family: &str) -> Vec<u8> {
    let (size, sha1) = if family == "Redump" {
        (11760, "89075aa74e5d8ec7bc850a0930d0deccf320a01f")
    } else {
        (3, "a9993e364706816aba3e25717850c26c9cd0d89d")
    };
    format!(r#"<datafile><header><name>{system}</name><author>{family}</author><version>20261010</version></header><game name="Known Game"><rom name="game.bin" size="{size}" sha1="{sha1}"/></game></datafile>"#).into_bytes()
}
fn zip(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in files {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
#[test]
fn catalog_batches_detect_contents_without_custom_systems_or_raw_defaults() {
    let staging = Staging::default();
    let archive = zip(&[
        ("renamed.txt", dat("Nintendo - Game Boy", "No-Intro")),
        ("playstation.dat", dat("PSX", "Redump")),
    ]);
    let preview = StandardCatalogInspector
        .inspect_upload(&archive, &staging, 512 * 1024 * 1024, 128)
        .unwrap();
    assert_eq!(preview.entries.len(), 2);
    assert_eq!(preview.member_count, 2);
    assert!(preview.entries.iter().all(|entry| entry.upload.is_some()));
    assert_eq!(
        preview.entries[1]
            .upload
            .as_ref()
            .unwrap()
            .metadata
            .byte_domain,
        "org.junk.game.cd.raw-2352-track"
    );
    for bytes in [
        dat("Imaginary console", "No-Intro"),
        dat("PS1", "Mystery convention"),
        b"<datafile><game".to_vec(),
    ] {
        let p = StandardCatalogInspector
            .inspect_upload(&bytes, &staging, 512 * 1024 * 1024, 128)
            .unwrap();
        assert!(p.entries[0].upload.is_none());
    }
}
#[test]
fn archive_and_expansion_limits_are_enforced_before_catalog_acceptance() {
    let staging = Staging::default();
    for archive in [
        zip(&[("../bad.dat", dat("GB", "No-Intro"))]),
        zip(&[("/bad.dat", dat("GB", "No-Intro"))]),
        zip(&[("nested.zip", zip(&[("game.dat", dat("GB", "No-Intro"))]))]),
    ] {
        assert!(
            StandardCatalogInspector
                .inspect_upload(&archive, &staging, 512 * 1024 * 1024, 128)
                .is_err()
        );
    }
    let archive = zip(&[
        ("one.dat", dat("GB", "No-Intro")),
        ("two.dat", dat("GB", "No-Intro")),
    ]);
    assert!(
        StandardCatalogInspector
            .inspect_upload(&archive, &staging, 512 * 1024 * 1024, 1)
            .is_err()
    );
    assert!(
        StandardCatalogInspector
            .inspect_upload(&archive, &staging, 128, 128)
            .is_err()
    );
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .add_symlink(
            "link.dat",
            "target.dat",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    let archive = writer.finish().unwrap().into_inner();
    assert!(
        StandardCatalogInspector
            .inspect_upload(&archive, &staging, 512 * 1024 * 1024, 128)
            .is_err()
    );
}
#[test]
fn nul_zip_member_names_are_rejected_without_staging() {
    let mut archive = zip(&[("bad.dat", dat("GB", "No-Intro"))]);
    let central = archive
        .windows(4)
        .position(|bytes| bytes == b"PK\x01\x02")
        .unwrap();
    // Both ZIP headers must describe the same invalid filename.
    for (header, name) in [(0, 30), (central, 46)] {
        archive[header + name] = 0;
    }
    let staging = Staging::default();
    assert!(
        StandardCatalogInspector
            .inspect_upload(&archive, &staging, 512 * 1024 * 1024, 128)
            .is_err()
    );
    assert!(staging.0.lock().unwrap().is_empty());
}
#[test]
fn conflicting_document_systems_require_a_constrained_explicit_choice() {
    let bytes = String::from_utf8(dat("PS1", "Redump"))
        .unwrap()
        .replace("<author>", "<description>Saturn</description><author>");
    let preview = StandardCatalogInspector
        .inspect_upload(
            bytes.as_bytes(),
            &Staging::default(),
            512 * 1024 * 1024,
            128,
        )
        .unwrap();
    let entry = &preview.entries[0];
    assert!(entry.upload.is_none());
    assert_eq!(entry.corrections.len(), 2);
    assert!(
        entry
            .corrections
            .iter()
            .all(|u| u.metadata.system == "Sony - PlayStation"
                || u.metadata.system == "Sega - Saturn")
    );
}
