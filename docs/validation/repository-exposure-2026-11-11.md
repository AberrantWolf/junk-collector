# Repository exposure review — 2026-11-11

The review covered the working tree based on `cac29e3` and 4,475 unique blobs
reachable from the local Git refs. The pattern scan checked common credential
formats, private-key markers, credentials in URLs and known deployment
identifiers. Credential-like matches were synthetic fixtures. Earlier commits
still contain installation details. The review did not inspect deployed services,
hosted logs, release archives or copies of the repository outside the local refs.

The working-tree cleanup removes installation identifiers from deployment examples
and dated records while retaining source and release provenance. The native
offline screenshot's embedded metadata was removed with identical decoded pixels.
Local configuration, deployment environment backups and browser test output are
excluded from the relevant Git or Docker inputs.

The HTTP middleware had included alternate media origins in the Content Security
Policy of anonymous responses. [The playback contract](../automatic-private-playback.md#trust-and-transfer-policy)
now limits that allowlist to browser documents served with a valid session. The
Chrome regression checks anonymous and forged-cookie requests, authenticated
document and asset responses, and navigation from another site with an existing
Strict session cookie. It also exercises actual pinned local audio, seeking,
logout and public media refusal.

Local and separate HTTPS playback, account recovery, pairing and the browser
smoke passed in Chrome outside the macOS sandbox. The broader browser run was
refused by the sandbox preflight. Workspace tests passed, with a separate
collector-server rerun on the final server code. Web type, bundle and model
checks, strict workspace Clippy, architecture links and container tooling checks
passed. The Docker daemon was unavailable for a live build-context check.
