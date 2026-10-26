# Standalone raw CD capture helper — 2026-10-25

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
