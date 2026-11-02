# Digital album adoption and automatic covers — 2026-11-02

[Subsequent complete-source publication review](layered-library-refresh-review-2026-11-02.md)
records the combined main publication and fresh qualification.

Source implementation for the reported partial-match adoption and missing-cover
workflows. No release, production deployment or collection mutation is included.

## Behavior and ownership

The application accepts optional explicit file-to-release-track assignments with
the existing digital album choice. It validates every file against the frozen Job,
every slot against the retained, length/digest-validated release snapshot, and a
complete one-to-one mapping. Inventory, source, metadata-authority, user and Job
fences remain in force. Assignments become the existing durable reviewed metadata
inputs, not invented recording matches. Manual metadata retains authority.

The review distinguishes recording associations from album coverage, shows raw
AcoustID scores with their limited meaning, and labels partial album evidence.
The editor includes every file, local/release durations, individual assignments,
an explicit filename-order fill and a confirmation cleared by each edit. Duplicate
or incomplete assignments cannot be applied. Complete release tracklists and equal
file/track counts remain required; subset layouts and album-only adoption are not
implemented. No shared fingerprint/matching algorithm or sibling dependency changes.

## Missing-cover regression

The previous real-browser journey checked the preview and adopted text, but did
not check effective saved artwork. A new check reproduced the reported failure:
the resolution completed with `outcome: null` and no effective artwork. Text had
already committed when the Job yielded for artwork acquisition. On resume, the
workflow read the outcome from the authority's deliberately empty observation
list and silently skipped further artwork work.

Resolution now recovers the outcome from the validated retained provider snapshot,
including when its text publication is already committed. Adding an absent cover
also advances an explicitly selected candidate to its enriched immutable version
only when all other metadata, ranking and source fields are equivalent. Earlier
versions remain intact; changed text or an already accepted cover does not qualify
for this advancement. An explicit manual artwork choice continues to win.

Adoption requests automatic front-cover acquisition in the same application-owned
workflow. The browser regression requires effective saved artwork, a readable
thumbnail, a `candidates` outcome, no artwork error, and no `missing_artwork` review
reason without clicking Find cover art or selecting an image. Optional artwork
failures remain partial outcomes; a preview alone is not proof of saved artwork.

## User-authorized live diagnostic

Read-only inspection used the mounted `AthenaAudioBook` folder and the user's
AcoustID application key inherited through a new fish process. The key was not
printed, written to repository files or included in diagnostic results. Requests
sent fingerprints and full durations, not audio or purchase facts. No source WAVs
were modified. Disposable raw results are local temporary diagnostics, not new
permanent collection provenance.

The diagnostic used the pinned Test2 algorithm over the first 120 seconds, with
the full WAV duration. Fingerprints and coordinates were cross-checked against the
production implementation for Chapter 1, About the Author, Special Thanks and
Closing Credits. This is an identity diagnostic, not a full collection integrity
or catalog verification claim.

Of 24 files, 22 returned no AcoustID candidates. About the Author returned score
1.0 linked to [the correct recording](https://musicbrainz.org/recording/edbd7252-c8f7-41cb-9a16-59c21ee8f5a4).
Special Thanks returned score 0.5661765 linked to the unrelated
[Timescape recording](https://musicbrainz.org/recording/e0496aa5-11b4-4c9f-86f7-db2d299ec8c2),
plus a 0.52288634 result without a MusicBrainz recording link. Closing Credits
returned no result in this run. These are dated live responses, not guarantees
that future provider responses will be identical.

[The correct release](https://musicbrainz.org/release/0d062b63-5cca-4b0b-8d3d-ebae05a566e4)
has 24 tracks in filename order, with all duration differences between 56 and 75
milliseconds. This strongly corroborates manual mapping; it does not turn the
other 23 tracks into AcoustID-recognized recordings. Lowering Collector's 0.5
threshold cannot recover candidates absent from these raw responses. No threshold
slider or new automatic matching heuristic is introduced.

## Validation

Initial workspace tests, strict Clippy, web/model/contract checks and all 30 browser
journeys passed for the mapping change before the missing-cover assertion exposed
the additional defect. The repaired mapping/automatic-cover browser journey passes.
Final workspace tests, strict workspace Clippy, format/architecture/link checks,
web type/build/model checks and generated API checks pass. Five existing Rust
ignores remain; optional tool/oracle environment flags were not set. The browser
matching journey uses actual Chrome with WAV/MP3 and independently decoded WAV
input. Desktop (1440 px) and mobile (390 px) screenshots were inspected.

All 30 named browser journeys were exercised on the repaired implementation, but
the combined run was **not a clean pass**: intake-review failed once with an import
publication revision conflict. An isolated rerun of intake-review passed, followed
by all five journeys that had been omitted after that failure (game-audio,
documents, backup-reminder, remote-media and storage). The failed selector receipt
is `/tmp/collector-adoption-final-checks.json`; the successful follow-up log is
`/tmp/collector-adoption-browser-followup.log`. The earlier mapping-only receipt
is `/tmp/collector-mapping-checks.json`. These are local working-tree checks, not a
sealed release qualification or a CI baseline.

The failing fixture (`collector-intake-review-6m9AYi`) retained a package frozen at
Item revision 1. A second reviewed physical copy advanced inventory to revision 2;
a sibling's publication then added evidence at revision 3. The unchanged grouped
publication guard correctly refused the stale revision because physical inventory
had also changed. Metadata/artwork authority was stored separately. This exposes
an independent copy-setup/admission race, now recorded in the single
[remaining-work queue](../remaining-work.md). Its policy was not loosened here.
No production state was involved. A clean combined release run remains required;
this rerun does not erase the initial failure.

The store regression also exercises missing/duplicate/foreign assignments,
all-track application, original bytes, stale edits, cancellation, restart and
reindex. Metadata refresh coverage checks artwork-only enrichment, retained source
versions, a distinct manual cover and refusal to advance changed text. Desktop
and mobile mapping screenshots are generated under `web/test-results/`.
