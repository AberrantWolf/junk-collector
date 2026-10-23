# HL-07C release and setup wording — 2026-10-20

## Delivered changes

Startup/setup/doctor output, web enrollment and certificate help now describe the
active web/Trash Amp clients. Obsolete Phono-specific Add server, fingerprint-entry
and trust-confirmation controls are removed from current instructions. Trash Amp
help was checked against the sibling's actual Slint Settings → Servers page and
connection implementation: Collector server URL, optional Owner-provided trust
file path and Connect / Reconnect. Browser approval, independently obtained trust
material, system HTTPS and owner-provided PEM trust remain distinct.

Current UI uses saved/kept for stored files and precise protection language for
manual choices, existing accounts/TLS, item/list removal and recovery. Replacement
review still explains explicit metadata/artwork resets; it does not promise all
settings survive every update. Technical `preservation_master`, `preserve_only`,
receipt fields, storage formats, retention policy and dated historical evidence
are unchanged.

README intake/listening instructions now match HL-07A/B and the existing Imports,
Prepare audio and persistent-player workflows. The previous statement that closing
album details stops playback was incorrect and is corrected. The operator/container
guides explain monitored MusicBrainz contact versus account requirements and avoid
implying setup resets existing accounts. This wording work does not qualify fresh
NAS installation/update or provider readiness; those remain HL-05A.

## Self-review and checks

Removed an incidental certificate-prefix assertion: that test already compares
the actual downloaded PEM, displayed fingerprint and startup fingerprint and uses
the PEM to validate TLS. Removed an incidental HTTP English-message assertion:
the test retains typed `prepared_media_invalid` and real access/range/media checks.
Accessible action names in real journeys follow the new Save package labels.
No new copy-only snapshot tests or functional compatibility wrappers were added.

Passed: TypeScript, production web build and rebuilt embedded Rust server;
single-account and multi-account browser enrollment through actual one-time native
credential issuance; certificate browser download, independent file/startup hash,
explicitly validated HTTPS, local approval, mobile help and failed-read retry.
Chrome is 153.0.8010.37. Certificate and approval screenshots were captured by the
existing acceptance journeys; the mobile certificate help was visually reviewed.
Workspace tests, workspace Clippy with warnings denied, architecture/link checks,
formatting, generated API consistency, whitespace review and the full browser smoke
suite all passed against the rebuilt assets.

All runtime journeys use temporary local installations. No live archive, account,
certificate trust store, NAS container or source inbox was changed. This delivery
removes retired-client instructions; it does not qualify OS-installed trust,
second-device networking, new native-client behavior or a production update.
