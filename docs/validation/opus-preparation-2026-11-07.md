# Opus preparation and cached library readiness — 2026-11-07

The isolated browser journey uses real Chrome and `opusenc` from opus-tools 0.2
with libopus 1.5.2. It configures the collection default in Administration and
imports stereo and mono digital material through the Imports page. The fixture
uses retained FLAC, MP3 and 24-bit PCM WAV files at their native rates. The
preparation step stays active after the originals publish. The journey changes
the default during preparation, verifies that the admitted import still
produces its three 96 kbps copies, and compares each preserved original's
SHA-256 digest to the input.

The same journey overrides the new default for a CD import. An already-open
Prepare audio page sees the import and a later title edit. Its 96 kbps filter
excludes the ready digital album before applying a one-row page limit. Both
albums qualify at 128 kbps. A two-album preview and batch exercise a controlled
encoder refusal followed by a successful retry using the real encoder. The
list updates when the outputs publish. Single-album preparation exercises
cancellation and retry at 192 kbps and downloads an Ogg Opus range. A
mislabelled MP3 preserves its bytes and reports the unsupported preparation.
The page is checked for horizontal overflow at 390 px.

The storage regression runs the preparation list with a SQLite authorizer that
denies full item and capture payloads, processing observations and recipes.
A detailed plan fails under that restriction. Cold and warm list reads succeed,
and the warm response carries the rebuilt revision. The browser also checks
that opening and filtering the list never requests an album or its plan.

This qualification covers the fixture sources, the real local encoder and
browser, and the application workflow. Prepared Opus uses lossy encoding.
Broader original codecs, device playback, and production deployment require
separate evidence.

## Regression checks

All 31 named browser journeys passed across the validation runs. The
Opus, integration, smoke, Browse, scans, work history, CD defaults, purchases,
digital matching and game import journeys were rerun on the server built after
the workspace advanced to `4410441`. The purchases fixture uses disposable
ports. Bounded checks now wait for the durable metadata outcome in the
integration and digital-matching fixtures.

Strict workspace Clippy, formatting, architecture and link checks, OpenAPI
and TypeScript contract checks, the web build and model checks passed.
The workspace test run on `4410441` plus this delivery passed 637 tests with
five existing ignores. It included the opt-in real FLAC checks.

## Review follow-up

A regression preserves a two-track CD without analysis and publishes just one
128 kbps derivative. The cached filter keeps the album visible and reports
incomplete coverage. After structural analysis, the cache reports both tracks.
The album leaves the filter once the second track is prepared.

Guided import retains the policy shown in its submitted plan when the saved
default changes. Command replay returns the same import and preparation profile.
The Chrome journey changes both the bitrate and scope after bulk import review
and before submission. The digital import still produces the three reviewed
96 kbps files.
Opening another review in the same tab fetches the updated 192 kbps setting.

The Opus and short smoke journeys passed against the rebuilt server after the
same-tab fix. CD settings and digital purchases also passed during follow-up
validation. Both new Rust regressions and the compact-list regression passed.
The full Rust workspace passed 639 tests with five existing ignores, including
real FLAC checks. Strict workspace Clippy, architecture and link checks,
formatting, web build and model checks, API contracts, and tooling checks passed.
The full Chrome integration journey passed on a standalone retry after a
browser shutdown timeout and a separate checkout's fixture port conflict.
