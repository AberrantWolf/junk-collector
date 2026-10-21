# Alpha.3 container publication — 2026-10-21

Preparation in progress. The requested release packages the post-alpha.2 fixes,
[portable full archive transfer](archive-export-2026-10-21.md) and
[explicit inbox cleanup](inbox-cleanup-2026-10-21.md). See
[release notes](../releases/v0.1.0-alpha.3.md) for operating scope and schema-21
whole-point rollback requirements.

The exact current-stable Rust 1.99.1 is installed for qualification. Development,
CI, Docker and README pins agree; no shared-library algorithms or Git pins change.
Architecture/format, TypeScript check, deterministic web build, container tooling,
release tooling and strict all-target Clippy passed on this candidate. Workspace
tests passed: **454 passed, zero failures, four existing opt-in qualification
tests ignored**. The optimized executable builds and reports `0.1.0-alpha.3`;
portable export/import commands are present. The actual cleanup browser/recovery
delivery passed before this version-only packaging.

The rebuilt server also passed the full actual Chrome 155.0.8059.85 smoke: setup,
login/expiry, virtual WebAuthn registration/login, role denial, inventory/artwork,
interrupted bulk upload/grouping, provider concurrency/cancellation/reopen, FLAC
preparation and actual playback/seek/advance/revocation. Collection lists passed
append/repeats/reorder/conflict/recovery/shuffle and mobile/keyboard use. These
are temporary local fixtures, not a repeat physical-device claim.

The signed tag, native platform results, public digest/channel verification and
isolated Portainer update/recovery results will be recorded as they complete.
No pending check is claimed passed. Production remains unchanged.
