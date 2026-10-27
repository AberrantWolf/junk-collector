# Standalone raw CD capture helper — 2026-10-25

## Automatic folder naming — 2026-10-27

`rip-cd.sh --auto DESTINATION [DRIVE]` now creates a unique generated capture
directory and optionally renames a completed capture using an unambiguous catalog
number. The lookup uses shared TOC identity, reads no audio and has a fifteen-second
wall deadline with no retries. Manual naming and the acquisition/refinement policy
below remain available. See [implementation and focused acceptance](cd-folder-naming-2026-10-27.md);
successful live-provider naming and physical-drive timing remain unqualified.

## Accepted Q-only tradeoff — 2026-10-27

The user accepts documented Q-subchannel imperfections for ordinary hobbyist
music archiving, with further Q-only refinement optional and AccurateRip used
to assess audio within its reported coverage. This does not claim that Q errors
are meaningless, that a capture is a perfect disc archive or that a small count
certifies manufacturing compliance. See the
[governing decision](../decisions/cd-audio-capture-tradeoff.md).

Operator-supplied redumper b736 output for `TKCA-10171-GHIBLI-SONGS-2` records
a BU40N with OmniDrive v1.0.4 completing `dump` and `dump::extra`, reporting
SCSI 0 samples, C2 0 samples, Q 31, and the helper skipping normal refinement.
Manual `refine --refine-subchannel --retries=100` passes corrected 11 and then
1 Q sector; a requested 8x pass corrected 3 more. The expected remainder is 16.
The verbose pass recovered LBA 141189 and reported the other unsuccessful
locations, including a cluster around LBA 161386–161843. A second BU40N with
the same reported firmware corrected none at default optimal speed or requested
8x. Requested 1x did not establish measured 1x operation; no final result from
that experiment is used here. These are operator observations, not optical
reads performed by the documentation change or independent audio verification.

The recorded lead-out LBA 298755 gives 66:23.4 of program area. For scale, 31
and 16 bad Q blocks correspond to about 4.67 and 2.41 per ten minutes averaged
over the disc, respectively. Historical StageTech mastering guidance gives
instrument-dependent bad-subcode averages, not a universal pressed-disc Q-error
limit. A consumer-drive capture, especially one refined across passes, is not a
calibrated manufacturing test; local clusters and boundary/control problems
still require their own interpretation. No AccurateRip outcome for this disc
was supplied or inferred.

This supersedes the earlier unrun qualification only for the observed Q-only
helper skip and named manual passes. Broader drive/format, interrupted optical
I/O and Collector import/audio-check acceptance remain unqualified by this
record. The historical validation entries below retain their original dates
and scope. The helper's existing runtime behavior is unchanged by this decision.

Documentation validation: `python3 scripts/check_architecture.py` passed the
architecture, local documentation-link and fixture-manifest checks; fish
`--no-config --no-execute` accepted the documented command without executing it;
scoped `git diff --check` passed. The required selector preview proposed infra
coverage for the complete working tree, which also contains unrelated Rust/web
and earlier helper changes. This delivery edits only documentation; unit,
Rust/web/browser and optical-drive checks were not rerun or claimed as passed
for those other changes. No capture bytes were modified.

## Conditional refinement — 2026-10-26

The user reports that a refinement pass can leave their drive dumping at 1x
until power-cycled. The helper now starts refinement only when the initial
`media errors` summary contains nonzero SCSI or C2 sample counts. Q counts
alone do not trigger refinement: upstream
[CD acquisition/refinement](https://github.com/superg/redumper/blob/main/cd/cd_dump.ixx)
only includes them in refinement eligibility with `--refine-subchannel`, which
this helper does not enable. Missing, duplicate or malformed SCSI/C2 summaries
stop with retained files and an explicit review message; they neither silently
claim a clean read nor start an unnecessary refinement pass. Extended-area
acquisition remains enabled. This does not add catalog verification.

`python3 -B scripts/test_rip_cd.py` passes five offline regression tests with
subcases for clean/Q-only skips, each sample-error kind, unreadable/duplicate
summaries, extended-area failure and refinement failure. The fake executable
records actual invocations and checks explicit drive reuse; retained master and
extra-sidecar bytes are checked. `bash -n`, help output inspection, architecture/
documentation links and diff whitespace checks pass. The selector preview treats
generic scripts as infra and proposes the full source set; focused shell/fixture
checks were selected for this isolated operator wrapper. Rust/web/container
suites were omitted because their sources, contracts and build inputs are unchanged.
No physical drive was accessed for this adjustment.

The operator previously supplied successful explicit-drive `dump::extra` and
`refine` output for `B0022532-02`, with zero corrections. That qualifies the manual
recovery described below, not physical acceptance of the new conditional helper.

## Follow-up: drive reopening after initial dump

The operator's `B0022532-02/disc.log` records build b736, `disk11` (BU40N),
13 audio tracks, SCSI 0 samples, C2 0 samples, Q 113, and a normal END after
240 seconds. All six raw/TOC/log files exist. The next `dump::extra` invocation
failed before logging with `no ready drives detected`. This establishes a
subsequent automatic drive-selection failure, not a failed initial optical read.
The cause of the drive's unavailability (for example a transient exclusive-access
or readiness failure) has not been established.

The helper now reads the initial selected device from its fresh log and passes
it explicitly to both later stages, preventing repeated automatic selection and
accidental selection of another disc. Failure messages name the stopped stage.
Offline regression checks cover automatic selection followed by explicitly
pinned later stages and rejection when the initial log lacks a device. Physical
recovery with an explicitly selected drive remains unrun. The original fixture
and no-physical-read records below describe the initial delivery.

The requested operator script creates `DESTINATION/NAME`, using `disc` as the
shared file stem. It runs `dump`, `dump::extra`, then `refine` with 100 retries
by default, and checks for nonempty SCRAM, STATE, SUBCODE, TOC and log outputs.
All additional outputs remain alongside them, including FULLTOC and CD-TEXT
when supplied by the tool/drive. No file-deletion or conversion step exists.

The installed redumper reports build b736; its help confirms these commands and
the image path/name, retries and optional drive arguments. Upstream
[command dispatch](https://github.com/superg/redumper/blob/main/redumper.ixx)
and [options](https://github.com/superg/redumper/blob/main/options.ixx)
were checked. Separate acquisition commands avoid the aggregate `disc`
workflow's split/hash/info stages. No firmware or drive configuration changes
are made by the wrapper beyond normal redumper acquisition behavior.

Validation: `bash -n` and help invocation passed. A temporary offline executable
fixture exercised paths/executable names with spaces; the acquisition sequence;
retention of extra FULLTOC/CD-TEXT sidecars; refusal of an existing destination
without modifying it; a failure after initial acquisition preserving original
bytes and stopping later stages; missing required output rejection; and folder
traversal rejection. These fixtures validate wrapper behavior, not optical reads
or sidecar structure. No fixture capture was represented as a real dump.

Architecture/documentation/fixture-manifest checks passed. `cargo fmt --all --
--check` encountered formatting differences in concurrently edited music
metadata/store Rust files outside this helper's changes; those files were left
untouched. Full Rust/web suites were not run for the standalone shell helper.

No physical disc was read. Drive-specific completeness, interrupted optical I/O,
actual unresolved-error handling and Collector import of a newly captured disc
remain unqualified. A successful command and nonempty files do not prove
error-free audio, structural validity or catalog verification. The operator
must review the retained redumper log and use Collector's independent analysis
and verification workflow. Existing core/web behavior is unchanged.
