# Library access during music imports — 2026-11-13

Source candidate based on `06fd764e9646d85099163a680d3f053c37c1bdf6`, with the
read-pool, publication, and Library refresh changes in this delivery. Validation
used local macOS fixtures, rebuilt debug binaries, and Chrome 153.0.8010.12.

- The guided-import HTTP fixture paused a real Capture projection transaction
  while it held the SQL writer. The authenticated Library request returned the
  existing row on its first attempt. The attachment filter saw the Capture
  only after commit.
- Store regressions exercised a role downgrade inside the read snapshot, reader
  pool exhaustion and recovery, cursor stability during summary writes, and
  cursor rejection after membership changed. The guided intake regression
  restarted between verified staging and the publication claim and completed
  the original Job.
- Real-server browser smoke, browse, storage, and intake-review journeys passed.
  Browse held the count while the first page rendered and exercised the
  visible-row refresh. Intake review included 31 real inbox imports under
  shared-provider load, reload, mobile layout, and terminal progress.
- The final workspace suite passed 651 tests with five existing ignores. Strict
  workspace Clippy, generated API verification, TypeScript, web build and models,
  and architecture and documentation link checks passed.

Earlier iterations exposed a recursive writer acquisition in receipt-backed
archive changes and a catalog recovery regression when read handles outlived
the final writer. The updated CLI and corruption fixtures passed after the
fixes. An early browser run served outdated assets after a TypeScript error
blocked the web build. The rebuilt assets passed the browser checks. A later
workspace run caught a change to stored-grant lookup for disabled accounts.
The final run passed the mail recovery regression with that contract restored.

Production import latency, remote-mount timing, and installed native client
behavior remain unmeasured by this delivery. The measurements used local
fixtures. No release or deployment was performed.
