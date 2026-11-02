# Beta.18 qualification and Fi update — 2026-11-02 JST

The user authorized GitHub failure repair, a clean beta release, Fi update and
subsequent cleanup of obsolete Fi experiments/versions. Beta.17 is the release
baseline; the entire candidate selects workspace tests/Clippy, tooling/contracts,
web models and all 30 browser journeys plus both native release-image gates.

GitHub run 41417303165 failed because the broad playback test synchronously
counted track rows while the new lazy track request was still pending. The test
now waits for exactly one disc and two logical tracks before checking their
representation IDs and exercising actual playback, seek and revocation.

The separately recorded grouped-intake race is repaired by establishing all
reviewed physical copies before freezing the first child revision. Store selection
returns bounded same-destination manifest pages, including unadmitted members;
application code owns physical/digital policy and deterministic copy identity.
Manual edits retain the existing strict publication fence. Deterministic cases
exercise distinct copies, staggered staging, failed retry and reindex.

Publication, image identity, live acceptance and cleanup results will be recorded
after the exact-source workflow completes. Preparation is not release evidence.
