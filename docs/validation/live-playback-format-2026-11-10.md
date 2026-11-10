# Live playback format — 2026-11-10

The application selects only readable inputs belonging to the selected item and
source. Published recipes must match the current recorded source fingerprint and
track coordinate. The CD path reads the preparation PCM window through authorized
package handles; WAV input uses the shared strict PCM reader. No hashing or
conversion runs during browse or library refresh. An active stream checks account,
collection access, item generation, and audio listener policy until encoding ends.

Normal authenticated HTTP delivers incremental FLAC or Ogg Opus, with bounded
pipe buffers, timeouts, and cancellation when the response closes. Each request
uses an independent FFmpeg process. Matching saved media uses the existing ranged
response. The player can restart a live stream at a requested offset; paused live
playback closes the stream and remembers its position in the current player.
Playback settings are browser local. See [configuration](../configuration.md#live-playback-format).

The focused HTTP acceptance ran with real FFmpeg and the independent `flac`
decoder. It compared the exact PCM suffix after a raw CD seek, read a concurrent
113 kbps Opus stream, verified saved FLAC selection and byte ranges, and used a
stalled encoder with a child process to confirm disconnect stops the group and
reaps the encoder. Anonymous and revoked
reads fail, and the normal listener refuses conversion when separate audio is
required. These checks do not claim catalog or audio verification of an archive.

Browser acceptance played unprepared CUE/BIN audio in both formats, sought by
starting a new stream, and checked source release on pause. The 390 px Account
settings flow saved a custom bitrate, kept controls within the viewport, and
restored the same preference in the player after reload. The digital purchase
journey also played live Opus from an MP3 and live FLAC from an unprepared
24-bit WAV with trailing bytes. Separate WebTransport and HTTPS fixtures
exercised existing saved-file playback.

Separate WebTransport and isolated HTTPS media transcoding, native player UI,
physical mobile devices, and release-image or production qualification are
outside this source delivery.


After the final encoder cleanup changes, the three affected Rust packages
passed 145 tests with one existing ignored subprocess fixture. Workspace Clippy,
workspace doctests, architecture and formatting, the generated API contract,
web type checking/build, and web model tests passed. An earlier workspace run
passed 641 unit and integration tests with five existing ignores; its doctest
phase failed to find a crate during the overlapping rebuild. The separate final
workspace doctest command passed.

The browser harness now starts each named journey visibly, runs independent
journeys after failure, and returns a failed result for the batch. Intake and
integration use disposable ports and bounded provider shutdown. A held HTTP
response and a stalled child are covered without launching a browser. The broad
integration run previously read an error response as an album while the server
reported database lock errors. That fixture read now uses the app's bounded
retry for typed catalog contention, and reports the actual API failure if it
cannot proceed. The intake journey logs the paced 31-album settlement explicitly.

The updated runner completed `cd-library`, `integration`, and `intake-review`
in one serial invocation with no automatic journey reruns. The intake journey
logged the provider release and completed all 31 imports. Harness regressions
proved that a held HTTP response shuts down and a subsequent journey executes
even when an earlier journey fails or times out.
