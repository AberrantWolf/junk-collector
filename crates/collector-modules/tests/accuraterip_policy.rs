use collector_core::{Digest, HashAlgorithm, PcmFrame, StereoSampleFrameOffset};
use collector_modules::music::{
    AccurateRipDatabaseIdentity, AudioChecksumVersion, AudioVerificationStatus,
    InterpretAccurateRipInput, interpret_accuraterip_summary,
};
use junk_libs_accuraterip::{
    CrcMatch, DbarFile, DbarResponse, ExpectedChecksum, OffsetCandidate, TrackCrc,
    TrackVerification, TrackVerificationStatus, VerificationStatus, VerificationSummary,
};

fn database() -> AccurateRipDatabaseIdentity {
    AccurateRipDatabaseIdentity {
        track_count: 2,
        accuraterip_id1: 0x0008_4264,
        accuraterip_id2: 0x001c_c184,
        cddb_id: 0x1911_7f03,
    }
}

fn dbar() -> DbarFile {
    DbarFile {
        responses: vec![DbarResponse {
            track_count: 2,
            ar_id1: 0x0008_4264,
            ar_id2: 0x001c_c184,
            cddb_id: 0x1911_7f03,
            tracks: vec![
                ExpectedChecksum {
                    confidence: 7,
                    checksum: 0x1111_1111,
                    checksum_450: 0,
                },
                ExpectedChecksum {
                    confidence: 9,
                    checksum: 0x2222_2222,
                    checksum_450: 0,
                },
            ],
        }],
    }
}

fn track(position: u8, checksum: u32, matched: bool) -> TrackVerification {
    let matched_crc = CrcMatch {
        pressing: 0,
        confidence: if position == 1 { 7 } else { 9 },
        checksum,
        version: junk_libs_accuraterip::ChecksumVersion::V1,
    };
    TrackVerification {
        position,
        computed: TrackCrc {
            v1: checksum,
            v2: checksum.wrapping_add(1),
        },
        v1_matches: matched.then_some(matched_crc).into_iter().collect(),
        v2_matches: vec![],
        sample_shift: Some(6),
        frame_450_support: false,
        status: if matched {
            TrackVerificationStatus::Verified
        } else {
            TrackVerificationStatus::Mismatched
        },
    }
}

fn summary(status: VerificationStatus, tracks: Vec<TrackVerification>) -> VerificationSummary {
    VerificationSummary {
        status,
        chosen_sample_shift: Some(6),
        tracks,
        ambiguous_offsets: vec![],
        evaluated_sample_shift_start: -6,
        evaluated_sample_shift_end: 6,
        v2_evaluated_sample_shifts: vec![0, 6],
        unavailable_leading_frames: 3,
        unavailable_trailing_frames: 4,
    }
}

fn input() -> InterpretAccurateRipInput {
    InterpretAccurateRipInput {
        database: database(),
        database_response_digest: Digest::sha256(b"frozen dbar"),
        input_digests: vec![Digest::sha256(b"disc.cue"), Digest::sha256(b"disc.bin")],
    }
}

#[test]
fn one_matching_track_with_incomplete_v2_coverage_is_not_whole_disc_verification() {
    let result = interpret_accuraterip_summary(
        &dbar(),
        &summary(
            VerificationStatus::Verified,
            vec![track(1, 0x1111_1111, true), track(2, 0x3333_3333, false)],
        ),
        input(),
    )
    .unwrap();

    assert_eq!(result.status, AudioVerificationStatus::IncompleteSearch);
    assert!(result.selected_offset.is_none());
    assert_eq!(result.tracks[0].status, AudioVerificationStatus::Verified);
    assert_eq!(
        result.tracks[1].status,
        AudioVerificationStatus::IncompleteSearch
    );
    assert_eq!(
        result.tracks[0].coverage.v1_evaluated_offset_start,
        StereoSampleFrameOffset(-6)
    );
    assert_eq!(
        result.tracks[0].coverage.v2_evaluated_offsets,
        vec![StereoSampleFrameOffset(0), StereoSampleFrameOffset(6)]
    );
    assert_eq!(
        result.tracks[0].coverage.unavailable_leading_frames,
        PcmFrame(3)
    );
    assert_eq!(
        result.tracks[0].coverage.unavailable_trailing_frames,
        PcmFrame(4)
    );
    assert_eq!(
        result.tracks[0].checksums[0].version,
        AudioChecksumVersion::AccurateRipV1
    );
    assert_eq!(result.tracks[0].checksums[0].confidence, Some(7));
    assert_eq!(
        result.tracks[0].checksums[0].checksum.algorithm,
        HashAlgorithm::AccurateRipV1
    );
}

#[test]
fn whole_disc_verification_requires_every_database_track_at_one_offset() {
    let result = interpret_accuraterip_summary(
        &dbar(),
        &summary(
            VerificationStatus::Verified,
            vec![track(1, 0x1111_1111, true), track(2, 0x2222_2222, true)],
        ),
        input(),
    )
    .unwrap();

    assert_eq!(result.status, AudioVerificationStatus::Verified);
    assert_eq!(result.selected_offset, Some(StereoSampleFrameOffset(6)));
    assert_eq!(result.algorithm_version, 1);
    assert_eq!(result.database_response_count, 1);
    assert_eq!(result.input_digests.len(), 2);
    result.validate().unwrap();
}

#[test]
fn complete_v1_and_v2_search_can_report_mismatch() {
    let mut value = summary(
        VerificationStatus::Mismatched,
        vec![track(1, 0xaaaa_aaaa, false), track(2, 0xbbbb_bbbb, false)],
    );
    value.chosen_sample_shift = None;
    value.v2_evaluated_sample_shifts = (-6..=6).collect();
    value.unavailable_leading_frames = 0;
    value.unavailable_trailing_frames = 0;
    let result = interpret_accuraterip_summary(&dbar(), &value, input()).unwrap();
    assert_eq!(result.status, AudioVerificationStatus::Mismatched);
    assert!(
        result
            .tracks
            .iter()
            .all(|track| track.status == AudioVerificationStatus::Mismatched)
    );
}

#[test]
fn edge_limited_search_cannot_report_database_mismatch() {
    let mut value = summary(
        VerificationStatus::Mismatched,
        vec![track(1, 0xaaaa_aaaa, false), track(2, 0xbbbb_bbbb, false)],
    );
    value.chosen_sample_shift = None;
    value.v2_evaluated_sample_shifts = (-6..=6).collect();
    let result = interpret_accuraterip_summary(&dbar(), &value, input()).unwrap();
    assert_eq!(result.status, AudioVerificationStatus::IncompleteSearch);
    assert!(
        result
            .tracks
            .iter()
            .all(|track| track.status == AudioVerificationStatus::IncompleteSearch)
    );
}

#[test]
fn ties_and_frame_450_candidates_stay_inconclusive() {
    let mut tied_tracks = vec![track(1, 0x1111_1111, true), track(2, 0x2222_2222, true)];
    for track in &mut tied_tracks {
        track.status = TrackVerificationStatus::Ambiguous;
    }
    let mut value = summary(VerificationStatus::AmbiguousOffsets, tied_tracks);
    value.chosen_sample_shift = None;
    value.ambiguous_offsets = vec![
        OffsetCandidate {
            sample_shift: -6,
            full_matches: 2,
            minimum_confidence: 7,
            total_confidence: 16,
            frame_450_matches: 0,
        },
        OffsetCandidate {
            sample_shift: 6,
            full_matches: 2,
            minimum_confidence: 7,
            total_confidence: 16,
            frame_450_matches: 0,
        },
    ];
    let result = interpret_accuraterip_summary(&dbar(), &value, input()).unwrap();
    assert_eq!(result.status, AudioVerificationStatus::AmbiguousOffsets);
    assert_eq!(
        result.ambiguous_offsets,
        vec![StereoSampleFrameOffset(-6), StereoSampleFrameOffset(6)]
    );
    assert!(result.selected_offset.is_none());

    let mut frame_tracks = vec![track(1, 0xaaaa_aaaa, false), track(2, 0xbbbb_bbbb, false)];
    frame_tracks[0].frame_450_support = true;
    let mut frame_only = summary(VerificationStatus::IncompleteSearch, frame_tracks);
    frame_only.chosen_sample_shift = Some(6);
    let result = interpret_accuraterip_summary(&dbar(), &frame_only, input()).unwrap();
    assert_eq!(result.status, AudioVerificationStatus::IncompleteSearch);
    assert!(result.selected_offset.is_none());
}

#[test]
fn empty_database_is_no_data_and_wrong_identity_fails_closed() {
    let empty = DbarFile::default();
    let mut no_data = summary(VerificationStatus::NoData, vec![]);
    no_data.chosen_sample_shift = None;
    no_data.v2_evaluated_sample_shifts.clear();
    let result = interpret_accuraterip_summary(&empty, &no_data, input()).unwrap();
    assert_eq!(result.status, AudioVerificationStatus::NoData);
    assert_eq!(result.database_response_count, 0);

    let mut wrong = database();
    wrong.accuraterip_id1 ^= 1;
    let mut wrong_input = input();
    wrong_input.database = wrong;
    assert!(
        interpret_accuraterip_summary(
            &dbar(),
            &summary(
                VerificationStatus::Verified,
                vec![track(1, 0x1111_1111, true), track(2, 0x2222_2222, true)],
            ),
            wrong_input,
        )
        .is_err()
    );
}

#[test]
fn partial_evidence_keeps_computed_offset_and_cannot_be_relabelled_as_mismatch() {
    let value = summary(
        VerificationStatus::Verified,
        vec![track(1, 0x1111_1111, true), track(2, 0x3333_3333, false)],
    );
    let mut result = interpret_accuraterip_summary(&dbar(), &value, input()).unwrap();
    assert_eq!(result.tracks[0].computed_offset, StereoSampleFrameOffset(6));
    assert_eq!(result.tracks[1].computed_offset, StereoSampleFrameOffset(6));
    assert!(result.selected_offset.is_none());
    result.status = AudioVerificationStatus::Mismatched;
    assert!(result.validate().is_err());
}

#[test]
fn inconsistent_database_matches_and_track_offsets_fail_closed() {
    let valid = summary(
        VerificationStatus::Verified,
        vec![track(1, 0x1111_1111, true), track(2, 0x2222_2222, true)],
    );
    for mutation in 0..5 {
        let mut value = valid.clone();
        match mutation {
            0 => value.tracks[1].sample_shift = Some(0),
            1 => value.tracks[0].v1_matches[0].confidence += 1,
            2 => value.tracks[0].v1_matches[0].pressing = 999,
            3 => value.tracks[0].computed.v1 ^= 1,
            _ => value.v2_evaluated_sample_shifts = vec![0],
        }
        assert!(
            interpret_accuraterip_summary(&dbar(), &value, input()).is_err(),
            "mutation {mutation}"
        );
    }
    let mut result = interpret_accuraterip_summary(&dbar(), &valid, input()).unwrap();
    result.selected_offset = Some(StereoSampleFrameOffset(0));
    assert!(result.validate().is_err());
}

#[test]
fn real_shared_search_results_preserve_partial_and_complete_disc_semantics() {
    use junk_libs_accuraterip::{DiscTrackSamples, VerificationOptions, verify_with_offsets};
    let tracks = vec![
        DiscTrackSamples {
            position: 1,
            samples: vec![1; 9000],
        },
        DiscTrackSamples {
            position: 2,
            samples: vec![1; 9000],
        },
    ];
    // Independent arithmetic-series oracle for constant packed samples. The first
    // track excludes its first 2939 samples, the last excludes its final 2940.
    let mut database = dbar();
    database.responses[0].tracks[0].checksum = (2940 + 9000) * 6061 / 2;
    database.responses[0].tracks[1].checksum = 6060 * 6061 / 2;
    for partial in [false, true] {
        if partial {
            database.responses[0].tracks[1].checksum ^= 1;
        }
        let summary = verify_with_offsets(
            &database,
            &tracks,
            VerificationOptions {
                max_sample_shift: 0,
            },
        );
        let result = interpret_accuraterip_summary(&database, &summary, input()).unwrap();
        assert_eq!(
            result.status,
            if partial {
                AudioVerificationStatus::Mismatched
            } else {
                AudioVerificationStatus::Verified
            }
        );
        assert_eq!(result.tracks[0].status, AudioVerificationStatus::Verified);
        assert!(
            result
                .tracks
                .iter()
                .all(|track| track.computed_offset == StereoSampleFrameOffset(0))
        );
    }
}
