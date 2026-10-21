# Focused home-server ingress review — 2026-10-21

Bounded source review covered listener/router composition, exact Origin/native
credential admission, account/session/enrollment authority, curation/upload body
handling and authorized media/asset/range delivery. Existing alpha.3 public-image,
NAS/Safari and actual-origin/operator evidence is reused; no production request
mutated state and no new tunnel or exhaustive abuse matrix was introduced.

## Finding and repair

JSON/byte extractors could read a curation request body before the handler checked
its caller. An anonymous caller at an allowed browser origin could therefore make
large intake requests consume their body before eventual authentication failure.
The regression uses a body stream that records any poll and fails immediately;
it reproduced a 400 after reading the body instead of an early 401. It allocates
no large payload and has no timing-dependent stall.

Runtime composition now applies an authentication guard to the actual curation
API routes and the application's administrator guard to administration API routes,
before body extractors. Public routes still depend only on explicit exposure.
Collection-specific roles, freshness, revision/input-bound receipts and command
checks stay in their application services. Handlers revalidate authority after
body handling; the early guard does not freeze or replace those checks. Existing
route-specific size limits remain effective. Public identity/features, login and
native enrollment retain their intentional bootstrap behavior.

The focused regression rejects anonymous intake/import bodies and a non-admin
administrative body without polling them. All **32 HTTP tests** pass, including
exposure/spoofed-header composition, Origin attacks, native enrollment/revocation,
role/private-field/media/range denial and authorized upload workflows.

## Other reviewed boundaries and acceptance

Media and asset handlers authorize before obtaining opaque read capabilities.
Range parsing uses recorded length/immutable digest, prepared GETs validate actual
bytes, and private metadata/originals retain role-based redaction. Store path
validation refuses unsafe relative names and symlinked package members. Browser
queue state grants no authority; the existing actual loaded-player revocation
journey remains accepted. Native completion secrets are hashed, issuance is
transactional and expiring, and accepted identity-retention/account-peer-global
limits remain bounded. No new concrete repair was identified in this bounded pass
outside the body-admission finding. This is an internal focused review, not an
independent external audit or a claim that every attack/storage model is qualified.

Required checks passed on Rust 1.99.1: architecture/documentation/fixtures, format,
**455 workspace tests** (zero failures, four existing opt-in cases ignored), strict
all-target Clippy, TypeScript check and deterministic web build. The server was
rebuilt. Actual Chrome 155.0.8059.85 passed setup/login/expiry/virtual WebAuthn,
intake/metadata/artwork, provider concurrency/cancellation, actual FLAC playback/
seek/advance/revocation, list editing/recovery/shuffle and keyboard/mobile checks.
Self-review checked layer order, omitted-route 404 behavior, unchanged bootstrap
and Origin policy, and continued handler/application authority checks. The fix is
source delivery after alpha.3; it is not retroactively in that published image.
