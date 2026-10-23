# Browser Now Playing metadata — 2026-10-23

The operator requested track information after the iPhone lock screen displayed
only the application name. This delivery uses the standard browser Media Session
API; it does not change audio formats, routes or authorization.

- Personal queued tracks carry optional album artist alongside existing title,
  album and artwork. Album playback uses presented metadata; playlist presentation
  supplies the same authorized album artist. Removed/unavailable list entries do
  not expose artist metadata. Older saved queues remain usable without that field.
- The browser adapter publishes title/artist/album and a same-origin authorized
  artwork URL. It never publishes a capability URL or account credential to OS
  metadata. Supported play/pause/stop, previous/next and seek actions call existing
  player behavior. Unsupported browser actions leave ordinary playback usable.
- Natural track changes update Now Playing. Stop/release and audio detachment
  clear metadata and action handlers; stale element events cannot update the
  active session. Position/state follow actual audio events rather than a timer.

Actual Chrome **156.0.8078.47**, real core/media executables and FLAC/MP3 fixtures
pass in separate-origin and same-origin-prefix hosting: exact title, artist and
album, fetchable artwork, play/pause/seek/previous/next actions, natural completion
advancing metadata, immediate stop cleanup and logout/detached-source cleanup.
The purchase browser journey also covers playlist artist/album metadata through
the real list API, alongside its existing import/playback/privacy/edit workflow.

The purchase fixture's Chrome service-worker request needs its generated
certificate SPKI allowlist in addition to the isolated Playwright context's TLS
exception for request interception. No OS trust or production certificate policy
is changed. Neither fixture qualifies public-CA trust or physical lock-screen UI.

**Actual iPhone lock-screen title/artist/album/artwork rendering and OS-button
behavior await the operator's report after the
[beta.5 NAS update](container-beta5-publication-2026-10-23.md).** Earlier operator-reported
locked track changes establish playback continuity, not this new metadata UI.
