//! TOC-only identity adapter for the standalone capture helper. Never opens audio.
use std::io::Read;
use std::path::Path;

fn sidecar(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(65_538)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 65_537 {
        return Err("TOC exceeds MMC response size".into());
    }
    Ok(bytes)
}

fn identity(path: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let full = path.with_extension("fulltoc");
    match std::fs::symlink_metadata(&full) {
        Ok(_) => junk_libs_disc::redumper::validate_full_toc_bytes(&sidecar(&full)?)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let tracks = junk_libs_disc::redumper::pcm::parse_audio_toc(&sidecar(path)?)?;
    // The shared reader exposes stereo frames relative to LBA 0; Disc IDs use
    // sectors including the 150-sector lead-in. Unsupported negative starts
    // retain the generated name, matching Collector's conservative policy.
    let offset = |frame: i64| -> Result<u32, Box<dyn std::error::Error>> {
        if frame < 0 || frame % 588 != 0 {
            return Err("unsupported TOC coordinate".into());
        }
        Ok(u32::try_from(frame / 588 + 150)?)
    };
    let toc = junk_libs_disc_id::Toc {
        first_track: tracks.first().ok_or("empty TOC")?.number,
        last_track: tracks.last().ok_or("empty TOC")?.number,
        leadout_sector: offset(tracks.last().ok_or("empty TOC")?.end_frame)?,
        track_offsets: tracks
            .iter()
            .map(|track| offset(track.start_frame))
            .collect::<Result<_, _>>()?,
    };
    let id = junk_libs_disc_id::musicbrainz_disc_id(&toc)?;
    Ok(serde_json::json!({"disc_id": id, "track_count": tracks.len()}))
}

fn main() {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() != 1 {
        eprintln!("Usage: collector-cd-identity DISC.toc");
        std::process::exit(2);
    }
    match identity(Path::new(&args[0])) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("CD identity unavailable: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toc(offsets: &[i32], leadout: i32) -> Vec<u8> {
        let count = u8::try_from(offsets.len()).unwrap();
        let mut bytes = vec![0, 0, 1, count];
        for (number, lba) in (1..=count)
            .zip(offsets.iter().copied())
            .chain(std::iter::once((0xaa, leadout)))
        {
            bytes.extend([0, 0x10, number, 0]);
            bytes.extend(lba.to_be_bytes());
        }
        let size = u16::try_from(bytes.len() - 2).unwrap().to_be_bytes();
        bytes[..2].copy_from_slice(&size);
        bytes
    }

    #[test]
    fn published_musicbrainz_vector_needs_only_toc_sidecar() {
        // https://musicbrainz.org/doc/Disc_ID_Calculation, six-track example.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("disc.toc");
        std::fs::write(&path, toc(&[0, 15213, 32164, 46442, 63264, 80339], 95312)).unwrap();
        assert_eq!(
            identity(&path).unwrap(),
            serde_json::json!({"disc_id": "49HHV7Eb8UKF3aQiNmu1GR8vKTY-", "track_count": 6})
        );
    }

    #[test]
    fn unsupported_or_incomplete_sidecars_cannot_supply_a_lookup_identity() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("disc.toc");
        let audio = toc(&[0], 75);
        std::fs::write(&path, &audio).unwrap();
        assert_eq!(identity(&path).unwrap()["track_count"], 1);
        std::fs::write(path.with_extension("fulltoc"), [0]).unwrap();
        assert!(identity(&path).is_err());
        std::fs::remove_file(path.with_extension("fulltoc")).unwrap();
        let mut data = audio.clone();
        data[5] = 0x14;
        for bytes in [
            data,
            audio[..audio.len() - 1].to_vec(),
            toc(&[-1], 75),
            vec![0; 65_538],
        ] {
            std::fs::write(&path, bytes).unwrap();
            assert!(identity(&path).is_err());
        }
    }
}
