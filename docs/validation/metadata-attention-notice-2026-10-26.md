# Metadata attention notice — 2026-10-26

The metadata review notice uses a heading and a semantic unordered list of the
existing review reasons, followed by “(Manual metadata always takes precedence.)”.
It remains conditional on having attention reasons. Metadata policy is unchanged.
Scoped spacing preserves the existing notice appearance on desktop and mobile.

Passed architecture/documentation checks, Rust formatting, workspace tests,
workspace Clippy with warnings denied, web type checking and the production web
build. Rebuilt `collector-server` after generating the embedded assets.

Full Chrome smoke acceptance passed against that binary, including metadata
review and mobile overflow checks. Visually inspected its 390px mobile metadata
review screenshot: the heading, “Missing artwork” bullet and separate precedence
note are visible and fit the notice. The first sandboxed attempt failed at Chrome
launch before browser assertions; the permitted retry completed successfully.

No publication or deployment is included.
