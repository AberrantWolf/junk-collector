# Digital album matching evaluation probe

Dated 2026-10-29. Companion to the [assessment](digital-album-matching-evaluation.md).
This is an isolated research executable, not a Collector module or test of the
production matching workflow. It deliberately reports defects in published
libraries; a zero exit status does not mean every evaluated dependency is suitable.

## Reproduce

Requires Git, Rust/Cargo, CMake and a C++ toolchain. Network access is needed to
fetch source/dependencies, but the executable makes no provider requests. Use a
fresh temporary directory. Run these commands from the Collector repository root:

```sh
eval_root=/tmp/jc-album-matching-reproduction
mkdir -p "$eval_root"
git clone --no-checkout https://github.com/attilagyorffy/chromaprint-next.git "$eval_root/chromaprint-next"
git -C "$eval_root/chromaprint-next" checkout 7a384eee09caf17b4f6241f481dd784c2e937d9a
git -C "$eval_root/chromaprint-next" submodule update --init --recursive
python3 - "$eval_root" <<'PYCODE'
from pathlib import Path
import re, sys
root = Path(sys.argv[1]) / "probe"
(root / "src").mkdir(parents=True, exist_ok=True)
text = Path("docs/digital-album-matching-probe.md").read_text()
for language, body in re.findall(r"```(toml|rust)\n(.*?)\n```", text, re.S):
    target = root / ("Cargo.toml" if language == "toml" else "src/main.rs")
    target.write_text(body + "\n")
PYCODE
CARGO_HOME="$eval_root/cargo-home" cargo run --release --manifest-path "$eval_root/probe/Cargo.toml"
CARGO_HOME="$eval_root/cargo-home" cargo test --manifest-path "$eval_root/chromaprint-next/Cargo.toml" -p chromaprint-test-suite --test fingerprint_comparison --test algorithm_comparison --test config_parity
CARGO_HOME="$eval_root/cargo-home" cargo test --manifest-path "$eval_root/chromaprint-next/Cargo.toml" -p chromaprint-next --lib --release
```

The first Cargo command creates a lockfile; retain it and add `--locked` for repeats.
The evaluation resolved api_bindium 0.5.3 and ureq 3.5.0. Future dependency resolution
can differ; use the session lockfile under `/tmp/jc-album-matching-eval/probe` when
available for that exact resolution. `head` and `published` are Cargo aliases for
two distinct sources of the same version-numbered crate.

To repeat the published-release regression suite, copy `chromaprint-test-suite`
to a sibling directory under that upstream checkout, add an empty `[workspace]`
to its manifest, and change its chromaprint-next dependency from the local path
to `"=0.1.0"`. Run the same three test targets with `--no-fail-fast`. This preserves
the native source/fixture locations and evaluates the published release against
current boundary cases. It produced 13 passes and nine failures, all failures
at the partial-frame debug assertion.

For the additional native KissFFT comparison, make another sibling copy of that
suite, retain the local-path dependency, add `[workspace]`, and make the build
script's `cfg.define("FFT_LIB", "kissfft")` unconditional. All 22 checks passed.
This changes the native FFT backend on macOS, not the OS/architecture tested.

Optional live-response parsing reads `release-live.json`, `search-live.json`,
`browse-live.json` and `discid-live.json` from `MATCHING_EVAL_ROOT`, defaulting to
`/tmp/jc-album-matching-eval`. Missing files are skipped; the synthetic schema
checks still run. The research captured four successful public responses, spaced
at least one second apart, with a contact-bearing User-Agent and an 8-MiB bound.
No AcoustID key or request was used. The captured CD release is *Dummy*, release
ID `76df3287-6cda-33eb-8e9a-044b5e15ffdd`; this does not qualify broad catalog coverage.

## Recorded results

```text
PARITY {"cases_per_algorithm":[32,32,32,32,32],"head_chunk_mismatches":0,"head_mismatches":0,"head_ms":140,"native_ms":319,"published_chunk_mismatches":80,"published_mismatches":[0,0,0,0,0]}
QUERY_ROUNDTRIP false
LIVE_RELEASE Ok("Dummy")
LIVE_SEARCH Ok(2)
LIVE_BROWSE Ok(2)
LIVE_DISCID Ok(2)
```

The timing fields are observations, not assertions or benchmark acceptance.
The source asserts current-source compatibility, and reports known published
streaming mismatches plus schema/query behavior without hiding them as passes.

## Manifest

```toml
[package]
name = "digital-album-matching-probe"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
head = { package = "chromaprint-next", path = "../chromaprint-next/chromaprint-next" }
published = { package = "chromaprint-next", version = "=0.1.0" }
chromaprint-test-suite = { path = "../chromaprint-next/chromaprint-test-suite" }
musicbrainz_rs = { version = "=0.14.0", default-features = false }
serde_json = "1"
url = "2"
```

## Independent probe source

```rust
use chromaprint_test_suite::ffi::CChromaprint;
use musicbrainz_rs::{
    entity::{BrowseResult, discid::Discid, release::Release, search::SearchResult},
    prelude::*,
};
use serde_json::json;
use std::time::{Duration, Instant};

fn audio(rate: usize, channels: usize, seconds: usize, kind: usize) -> Vec<i16> {
    let mut state = 0x72ac019du32;
    (0..rate * seconds)
        .flat_map(|frame| {
            let t = frame as f64 / rate as f64;
            let section = frame / (rate / 3);
            let root = [110., 146.83, 196., 261.63, 164.81, 220., 130.81][section % 7];
            let env =
                ((frame % (rate / 3)) as f64 / (rate / 3) as f64 * std::f64::consts::PI).sin();
            let v = (root * t * std::f64::consts::TAU).sin() * 11000.
                + (root * 1.5 * t * std::f64::consts::TAU).sin() * 6000.;
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let noise = ((state & 65535) as i32 - 32768) as f64 * 0.08;
            (0..channels).map(move |channel| {
                if kind == 1 && (frame < rate * 2 || (frame / rate) % 7 == 0) {
                    0
                } else {
                    ((v * env + noise) * if channel == 0 { 1. } else { -0.65 })
                        .clamp(-32768., 32767.) as i16
                }
            })
        })
        .collect()
}

fn main() {
    let mut totals = [0usize; 5];
    let mut failures = [0usize; 5];
    let mut published_chunk_failures = 0;
    let mut head_failures = 0;
    let mut stream_failures = 0;
    let mut native_time = Duration::ZERO;
    let mut head_time = Duration::ZERO;
    for rate in [8000, 11025, 22050, 44100, 48000, 88200, 96000, 192000] {
        for channels in [1, 2] {
            for kind in [0, 1] {
                let samples = audio(rate, channels, 12, kind);
                for algorithm in 0..5 {
                    let ha = [
                        head::Algorithm::Test1,
                        head::Algorithm::Test2,
                        head::Algorithm::Test3,
                        head::Algorithm::Test4,
                        head::Algorithm::Test5,
                    ][algorithm];
                    let pa = [
                        published::Algorithm::Test1,
                        published::Algorithm::Test2,
                        published::Algorithm::Test3,
                        published::Algorithm::Test4,
                        published::Algorithm::Test5,
                    ][algorithm];
                    let tick = Instant::now();
                    let c = CChromaprint::new(algorithm as i32);
                    c.start(rate as i32, channels as i32);
                    c.feed(&samples);
                    c.finish();
                    native_time += tick.elapsed();
                    let raw = c.get_raw_fingerprint();
                    assert!(!raw.is_empty());
                    let encoded = c.get_fingerprint();
                    let hash = c.get_fingerprint_hash();
                    let tick = Instant::now();
                    let mut h = head::Fingerprinter::new(ha);
                    h.start(rate as u32, channels as u16).unwrap();
                    h.feed(&samples).unwrap();
                    h.finish().unwrap();
                    head_time += tick.elapsed();
                    let mut p = published::Fingerprinter::new(pa);
                    p.start(rate as u32, channels as u16).unwrap();
                    p.feed(&samples).unwrap();
                    p.finish().unwrap();
                    totals[algorithm] += 1;
                    if raw != p.fingerprint() || encoded != p.encode() || hash != p.hash() {
                        failures[algorithm] += 1;
                    }
                    let mut ps = published::Fingerprinter::new(pa);
                    ps.start(rate as u32, channels as u16).unwrap();
                    for chunk in samples.chunks(997) {
                        ps.feed(chunk).unwrap();
                    }
                    ps.finish().unwrap();
                    if ps.fingerprint() != raw || ps.encode() != encoded {
                        published_chunk_failures += 1;
                    }
                    if raw != h.fingerprint() || encoded != h.encode() || hash != h.hash() {
                        head_failures += 1;
                        println!(
                            "HEAD_MISMATCH rate={rate} ch={channels} kind={kind} alg={algorithm}"
                        );
                    }
                    let mut s = head::Fingerprinter::new(ha);
                    s.start(rate as u32, channels as u16).unwrap();
                    for chunk in samples.chunks(997) {
                        s.feed(chunk).unwrap();
                    }
                    s.finish().unwrap();
                    if s.fingerprint() != raw || s.encode() != encoded {
                        stream_failures += 1;
                    }
                }
            }
        }
    }
    println!(
        "PARITY {}",
        json!({"cases_per_algorithm":totals,"published_mismatches":failures,"published_chunk_mismatches":published_chunk_failures,"head_mismatches":head_failures,"head_chunk_mismatches":stream_failures,"native_ms":native_time.as_millis(),"head_ms":head_time.as_millis()})
    );
    // Empty/short fingerprints are not identifications.
    for n in [0, 1, 11025, 33075] {
        let mut h = head::Fingerprinter::new(head::Algorithm::Test2);
        h.start(11025, 1).unwrap();
        h.feed(&vec![0; n]).unwrap();
        h.finish().unwrap();
        println!(
            "SHORT samples={n} fingerprint_len={}",
            h.fingerprint().len()
        );
    }
    for (rate, ch) in [(0, 1), (44100, 0), (100, 1)] {
        let mut h = head::Fingerprinter::new(head::Algorithm::Test2);
        println!(
            "INVALID rate={rate} ch={ch} rejected={}",
            h.start(rate, ch).is_err()
        );
    }
    let client = musicbrainz_rs::client::MusicBrainzClient::new(
        "JunkCollectorResearch/0.1 (https://github.com/AberrantWolf/junk-collector)",
    );
    let lookup = Release::fetch()
        .id("76df3287-6cda-33eb-8e9a-044b5e15ffdd")
        .with_recordings()
        .with_artist_credits()
        .with_release_groups()
        .as_api_request(&client)
        .unwrap();
    println!("LOOKUP {}", lookup.uri());
    let browse = Release::browse()
        .by_recording("af40d6b8-58e8-4ca5-9db8-d4fca0b899e2")
        .limit(100)
        .offset(100)
        .with_recordings()
        .as_api_request(&client)
        .unwrap();
    println!("BROWSE {}", browse.uri());
    let search = Release::search("release:\"A & B\" AND artist:\"X/Y\"".to_owned())
        .limit(25)
        .as_api_request(&client)
        .unwrap();
    println!("SEARCH {}", search.uri());
    let search_url = url::Url::parse(&search.uri().to_string()).unwrap();
    println!(
        "QUERY_ROUNDTRIP {}",
        search_url
            .query_pairs()
            .any(|(key, value)| key == "query" && value == "release:\"A & B\" AND artist:\"X/Y\"")
    );
    let root = std::env::var("MATCHING_EVAL_ROOT")
        .unwrap_or_else(|_| "/tmp/jc-album-matching-eval".into());
    let live = std::fs::read(format!("{root}/release-live.json"));
    if let Ok(live) = live {
        println!(
            "LIVE_RELEASE {:?}",
            serde_json::from_slice::<Release>(&live).map(|r| r.title)
        );
    }
    let release = json!({"id":"00000000-0000-4000-8000-000000000001","title":"Example", "media":[{"id":"medium", "track-count":1, "position":1,"format":"Digital Media","tracks":[{"id":"track","title":"Track","number":"1","position":1,"length":180000,"recording":{"id":"recording","title":"Recording","length":180000}}]}]});
    let r: Release = serde_json::from_value(release.clone()).unwrap();
    let t = &r.media.as_ref().unwrap()[0].tracks.as_ref().unwrap()[0];
    assert_eq!(t.recording.as_ref().unwrap().id, "recording");
    assert_eq!(t.length, Some(180000));
    println!("SCHEMA full_release=ok");
    let mut minimal = release.clone();
    minimal["media"][0].as_object_mut().unwrap().remove("id");
    println!(
        "SCHEMA missing_medium_id={:?}",
        serde_json::from_value::<Release>(minimal)
            .err()
            .map(|e| e.to_string())
    );
    let mut language = release.clone();
    language["text-representation"] = json!({"language":"qzz","script":"Qzzz"});
    println!(
        "SCHEMA unknown_language_script={:?}",
        serde_json::from_value::<Release>(language)
            .err()
            .map(|e| e.to_string())
    );
    let browse = json!({"release-count":1,"release-offset":0,"releases":[release.clone()]});
    assert!(serde_json::from_value::<BrowseResult<Release>>(browse.clone()).is_ok());
    let mut browse_extra = browse;
    browse_extra["future-field"] = json!(true);
    println!(
        "SCHEMA browse_extra={:?}",
        serde_json::from_value::<BrowseResult<Release>>(browse_extra)
            .err()
            .map(|e| e.to_string())
    );
    let search =
        json!({"created":"2026-10-29T00:00:00Z","count":1,"offset":0,"releases":[release.clone()]});
    println!(
        "SCHEMA search={:?}",
        serde_json::from_value::<SearchResult<Release>>(search)
            .err()
            .map(|e| e.to_string())
    );
    let disc =
        json!({"id":"disc","offset-count":1,"sectors":12345,"offsets":[150],"releases":[release]});
    assert!(serde_json::from_value::<Discid>(disc).is_ok());
    println!("SCHEMA discid=ok");
    if let Ok(live_search) = std::fs::read(format!("{root}/search-live.json")) {
        println!(
            "LIVE_SEARCH {:?}",
            serde_json::from_slice::<SearchResult<Release>>(&live_search).map(|v| v.entities.len())
        );
    }
    if let Ok(live_browse) = std::fs::read(format!("{root}/browse-live.json")) {
        println!(
            "LIVE_BROWSE {:?}",
            serde_json::from_slice::<BrowseResult<Release>>(&live_browse).map(|v| v.entities.len())
        );
    }
    if let Ok(live_discid) = std::fs::read(format!("{root}/discid-live.json")) {
        println!(
            "LIVE_DISCID {:?}",
            serde_json::from_slice::<Discid>(&live_discid)
                .map(|v| v.releases.unwrap_or_default().len())
        );
    }
    assert_eq!(head_failures, 0);
    assert_eq!(stream_failures, 0);
}
```
