# Security dependency review checkpoint — 2026-10-25

Scope: review the accumulated dependency, ZIP, shared XML/CHD and PEM-parser
changes, fix issues found and commit that work before the separately authorized
Utoipa 6 upgrade. Unrelated NAS deployment documentation edits are retained in
the working tree and excluded from this security commit.

## Finding and correction

The shared DAT reader claimed a 64 KiB token/event bound but reset its counter
on literal `<`/`>` bytes. XML allows those delimiter bytes inside comments and
CDATA, and `>` inside quoted attributes. A generated game with an ignored
attribute containing 65,537 `>` bytes parsed successfully before the fix. This
could let the XML parser buffer a much larger event before consumer-side limits
ran; ignored comments had no decoded-value check afterward.

Published shared commit `317517c430744e4f1196e30850b37b3f13c10560` on
`codex/bound-dat-xml-events` resets the budget only when quick-xml returns an
event. `BufRead::fill_buf` exposes at most the remaining event budget and errors
before supplying additional bytes. The 64 MiB document cap, ClrMamePro line
bound and 64 KiB accumulated decoded-text cap remain separate. Oversized quoted
attributes, ignored comments and CDATA now fail before event buffering can grow
past the limit, regardless of embedded delimiter bytes. Ordinary entity/text
handling remains covered by the existing vectors. Collector's seven shared pins
move together to that commit.

## Other reviewed changes

- The lockfile retains the targeted patched versions of Quinn/Tokio, rustls,
  certificate validation/root certificates, ZIP, quick-xml and CHD/Zstandard.
  No advisory ignore, generic TLS bypass or dependency fork is introduced.
- ZIP continues in-memory inspection with path/type/encryption, member and
  expanded-byte rejection. The explicit absolute-path rejection preserves the
  stricter application policy despite ZIP 9's changed enclosed-name behavior.
  No filesystem extraction API is used.
- PEM trust still requires exactly one certificate with the authenticated DER
  pin. The checked DER is passed directly to TLS; builder validation and the
  enrolled server identity check still precede credential use. Saved control
  trust and remote system-HTTPS handling are unchanged.
- Shared CHD tests retain independent chdman/raw-track comparisons and their
  stated synthetic-fixture limits. They do not claim a full malicious-input or
  physical-media qualification.

No additional actionable regression was found in these changed paths. This is
a focused code/dependency review, not a comprehensive audit of the entire app or
every transitive dependency. Prior full-workspace, native TLS and browser evidence
is retained in the individual delivery records.

## Review validation

- Reproduced the quoted-attribute limit bypass with the new public-parser test
  failing before the fix; after the correction all 11 shared DAT parser tests,
  doc tests, all-targets/all-features Clippy and workspace formatting pass.
- Retro Junk DAT/catalog/import/archive-import suites passed against the corrected
  shared parser in the isolated sibling snapshot.
- Trash Amp Collector unit/real-server/doc tests passed against the correction
  through temporary local source patches; its existing ignored cases remain
  ignored. Primary sibling source/lockfiles are untouched.
- Collector's `standard_catalogs` tests and provider/client all-targets Clippy
  pass against the published correction.
- The pre-upgrade OpenAPI/TypeScript contract check passed. The generated baseline
  was retained for semantic comparison during the Utoipa phase.
- The refreshed RustSec audit scans 413 locked dependencies with yanked-package
  checks, reports zero vulnerabilities and only `paste`'s maintenance warning.

- The rebuilt-server `npm --prefix web run test:game-catalog` journey passed:
  mixed DAT/ZIP upload, entity-containing metadata, activation, immediate refresh,
  manual authority, future import and malformed-input refusal.
- Architecture/documentation-link/fixture-manifest checks, Rust formatting and
  diff whitespace checks passed.

The subsequent Utoipa phase will run the complete repository gates against the
combined final source. No image publication or live deployment is included.
