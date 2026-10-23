# AccurateRip contribution investigation — HL-08

Researched 2026-10-18. **Decision: use an existing supported Windows ripper with
the physical disc; do not implement Collector submission from stored checksums.**
The bounded investigation is complete. Submission support remains unimplemented.
No results were submitted and no provider was contacted on the user's behalf.

## Supported route and access

AccurateRip's public [software list](https://www.accuraterip.com/software.htm)
lists compatible readers, including Mac applications. Compatibility with database
lookup does not establish contribution support. Maintainer Spoon's
[13 April 2025 reply, post 2](https://forum.dbpoweramp.com/forum/dbpoweramp/cd-ripper/331574-native-linux-program-that-supports-submitting-to-accuraterip-ctdb-or-similar)
identifies **EAC and dBpoweramp** as the submitting applications. His
[12 July 2025 reply, post 8](https://forum.dbpoweramp.com/forum/dbpoweramp/cd-ripper/38534-submit-results-to-accuraterip-from-macos)
confirms dBpoweramp for Mac still does not submit. The earlier
[4 July 2023 explanation, post 12](https://forum.dbpoweramp.com/forum/dbpoweramp/cd-ripper/29194-submitting-data-to-accuraterip-from-a-mac)
distinguishes Mac drive-name lookup from self-detected offset calibration.

| Candidate | Finding and recommendation |
| --- | --- |
| dBpoweramp CD Ripper on Windows | Supported contribution route. Its [official help](https://doc.dbpoweramp.com/dmc/en/CDHelp.htm) describes the periodic request to send ripping results. Use its built-in submission workflow. |
| Exact Audio Copy on Windows | Supported alternative named by the maintainer. Use its AccurateRip ripping/submission workflow; this investigation did not execute or qualify either ripper. |
| Mac dBpoweramp, XLD, native Linux tools, Collector | No supported submission route established by the reviewed sources. The Mac compatibility list is not a submission promise. A Mac-only installation is therefore a practical blocker for this recommendation. |
| CUETools/CUERipper and CTDB | AccurateRip lookup is distinct from CTDB contribution. [CUETools' CTDB documentation](https://cue.tools/wiki/CUETools_Database) describes submission to a separate database; it does not establish an AccurateRip write route. |

AccurateRip Meta is also separate: title/artwork submissions do not contribute
audio checksums. EAC's [July 2024 release note](https://exactaudiocopy.de/whats-new/)
about missing unknown-CD submission concerns that metadata service, not a reason
to reinterpret the maintainer's list of checksum-submitting clients.

The [AccurateRip access statement](https://www.accuraterip.com/) allows
non-commercial use and requires prior agreement for commercial use. It does not
publish a general third-party submission contract. No supported Collector write
endpoint, authentication scheme, payload schema or acceptance contract was found
in the reviewed public material. This is a bounded finding, not proof that a private
developer arrangement is impossible. A future integration would first need a
supported agreement/specification from Illustrate, whose
[contact route](https://www.dbpoweramp.com/email) is public. Reading the database
does not establish permission or eligibility to write to it.

## Prerequisites and evidence

The recommended route needs the owned physical CD, a working optical drive on
Windows, a supported ripper installation and network access for AccurateRip.
Use the ripper's own contribution queue, not a virtual drive mounted from the
Collector package. The maintainer has explicitly described
[virtual-drive submissions as grounds for rejection, 5 June 2019](https://forum.dbpoweramp.com/forum/dbpoweramp/cd-ripper/38564-accuraterip-rip-is-not-getting-accepted-into-ar-db-how-come).

**Documented configuration requirement:** let AccurateRip establish the actual
drive's read offset using suitable known discs. The
[dBpoweramp setup guide](https://doc.dbpoweramp.com/dmc/en/cd-ripper-setup-guide.htm)
normally requires one suitable disc for a known drive, or three matching results
from different discs for an unknown/purged drive. A missing-database target disc
cannot serve as its own known reference. Configure secure ripping, cache handling
and C2 according to that drive's capabilities; save the extraction log. A manually
entered table value or Collector's comparison offset is not proof that this
calibration completed.

**Local preservation/acceptance policy, not a claimed submission wire schema:**
retain the ripper/version, drive identification/firmware when reported, calibration
outcome and offset, original full TOC/track boundaries/lead-out, all track results
and checksums, read-error/retry report, and the submission time/outcome. Read all
tracks securely and retain unprocessed lossless output separately. Do not treat
an insecure/incomplete extraction as eligible merely because CRCs can be computed.
Avoid audio-changing DSP when retaining a source intended for later comparison.
The ripper decides what it can collect and send; these records do not grant
Collector eligibility to synthesize its queue or its acceptance result.

An existing queued contribution from a supported ripper may be usable through that
same installation. Stored audio, a CUE/TOC, a redumper log or Collector observations
alone do not establish that such a queue exists. If the physical disc or supported
configured ripper is unavailable, record that operational blocker rather than
fabricating drive/rip history or replaying guessed requests.

## Feasible operator workflow

1. Keep the existing Collector master and evidence unchanged. On Windows, use
   dBpoweramp CD Ripper or EAC with the physical disc and complete AccurateRip drive
   configuration first. Missing calibration is a stop condition.
2. Securely extract the complete disc to a separate location and retain its full
   extraction report. Inspect each track for unrecovered errors. Missing reference
   data is distinct from a read failure and is not proof of accuracy; dBpoweramp's
   [status descriptions](https://doc.dbpoweramp.com/dmc/en/CDHelp.htm) distinguish
   secure/no-database results from insecure extraction.
3. Review the ripper's built-in submission prompt/command and obtain explicit user
   authorization before sending results. Let that ripper submit its own collected
   results. Preserve the reported outcome. “Sent” does not prove acceptance,
   visibility in a lookup, or independent verification.
4. Allow for provider processing/publication. The maintainer describes
   [automated processing without individual-submission tracking](https://forum.dbpoweramp.com/forum/dbpoweramp/cd-ripper/38564-accuraterip-rip-is-not-getting-accepted-into-ar-db-how-come).
   There is no acceptance deadline established here. Do not repeatedly submit to
   force publication or simulate independent contributors by re-ripping.
5. Later, in Collector's existing music checks, use **Fetch AccurateRip response**
   for the recorded original layout. Inspect/select the newly acquired matching
   response and run the audio check against it. Keep the original dated no-data
   observation; a fetched response alone is not a new audio result.
6. Account for self-matches. The
   [PerfectTUNES help](https://www.dbpoweramp.com/help/PerfectTUNESOSX/accuraterip)
   recommends confidence of at least two after contributing, because confidence
   one can match your own submission. Confidence is aggregate evidence, not an
   identity list proving independent contributors. Collector does not currently
   track that the operator submitted a rip; do not interpret a later matching
   result as independent corroboration solely because it is displayed as verified.

For **SQ Relax - Krystelle's Chillwave Vol. 2**, use the same physical pressing.
The [2026-10-10 lookup record](validation.md#sq-relax-vol-2-live-accuraterip-lookup--2026-10-10)
established 21 tracks, lead-out LBA 312008 and IDs
`00350715-03223214-19104015`. Compare the new full TOC/identity with that record;
artist/title agreement is insufficient. That dated 404 is not a claim about today's
database contents. This investigation did not re-query that disc, read its audio,
re-rip it, or transmit its results.

## Collector integration assessment

The current [provider adapter](../crates/collector-providers/src/accuraterip.rs)
performs bounded HTTPS GET acquisition of DBAR responses. Its only accepted
operation is acquisition. The
[music evidence types](../crates/collector-modules/src/music.rs) retain disc IDs,
input digests, checksum versions, computation offsets, coverage and confidence.
[Capture](../crates/collector-core/src/lib.rs) and
[source fingerprints](../crates/collector-core/src/processing.rs) bind preserved
files and observations. Uploaded logs may remain intact as package files, but
these typed records do not attest to optical-drive identity, measured calibration,
secure physical extraction or an approved submission session. An offset found by
comparison with a database pressing must not be relabeled as the drive's read
offset. No contribution command, receipt or supported transport is implemented.

Direct integration is therefore blocked by both provider support/contract and
missing typed submission provenance. It is not a matter of posting existing CRCs.
If separately requested after a supported provider agreement, first settle required
fields, eligible sources/calibration, consent, private contributor identity,
idempotence/uncertain-send recovery, and self-match interpretation. Application
services would own that policy, modules evidence semantics, adapters transport,
and shared libraries the existing checksum algorithms. Do not build a second
verification pipeline or a generic retention engine for this investigation.

## Evidence limits

Sources were reviewed on 2026-10-18. The July 2025 Mac reply was available in indexed
publisher text; direct fetching returned HTTP 403. It is corroborated by the older
maintainer explanation, but neither source is a test of a locally installed app.
No supported Windows/drive setup was exercised, provider acceptance was not
demonstrated, and no submission permission was requested. Exact private payload
fields, identity/deduplication rules and current publication thresholds remain
unestablished. These limits do not prevent the documented external-tool
recommendation; they prevent claiming an implemented/qualified Collector uploader.

See [dated validation](validation/accuraterip-contribution-2026-10-18.md).
