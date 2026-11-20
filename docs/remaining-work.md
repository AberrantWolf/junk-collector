# Remaining work

## Open items

- **Shutdown under load:** stop blocking audio work within the container's stop grace
  period without cancelling durable requests, and resume it after restart.
- **Release receipt race:** reproduce the source-receipt visibility race in the release
  pipeline before changing its recovery logic.
- **Native client reliability:** test enrollment, fast account/server switching,
  output-device loss, offline LAN use and interrupted downloads on supported clients;
  check offline pinning and storage-budget behavior.
- **Format coverage:** add owned or legal reference material for advertised formats,
  especially whole-disc AccurateRip coverage.
- **Direct/private media:** test browser permission and UDP denial, long-track grant
  renewal, network changes, certificate renewal and restart on supported clients.
- **Deployment checks:** restart, interrupted upload/publication/encoding, missing
  tools, permissions and restore on the intended host layout; for public use, one
  trusted-HTTPS sign-in, playback and anonymous-API denial check.
- **HTTPS frontend isolation:** review firewall and egress restrictions for the TLS
  frontend, and confirm container resource limits are actually enforced by the host.
- **Intake I/O:** avoid the extra inbox → private → archive copy while keeping
  cancellation, cross-filesystem publication and retry.
- **Performance:** measure startup, browse, search, import and preparation on real
  storage and fix slow paths that show up. A longer-term goal is warm browse/search
  under 500 ms p95 with 100,000 items and five users on a modest four-core host; it is
  not a release gate.

AccurateRip submission is not planned in Collector: submit results with an existing
supported ripper using the physical disc.

## Possible later work

These need a concrete need and a design before starting:

- Retention: pruning old revisions, candidates, receipts and upload spools.
- Game metadata and artwork providers; upload resume; more document formats.
- Delta export and migration from legacy collections (raw-file import is not migration).
- Offline activity sync and pushed events.
- General relay/failover and authenticated trust rotation.
- Synchronized listening, more original codecs (AAC/ALAC/Opus), album-image splitting,
  smart lists, mobile apps, PriceCharting, video, emulators, ripping hardware,
  replication, runtime plugins and cross-collection deduplication.
- Persistent or distributed rate limiting.
