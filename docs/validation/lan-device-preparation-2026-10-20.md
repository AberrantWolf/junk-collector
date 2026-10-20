# Physical LAN browser preparation and acceptance — 2026-10-20

**Physical Safari trust, playback and Curator editing passed; Viewer/revocation
observations are in progress. This does not close parent HL-04.** The operator
used an iPhone 16 Pro, reported iOS 26.6.2, in Safari against a separate NAS
installation. No production account/config/media/ingress was changed.

## Prepared fixture and desktop preflight

The Synology DS1522+ runs the previously qualified public alpha.2 digest
`sha256:f86f46d2c8870f8d647fc9225ce2cfe7c4319521a29c89897b419abb77629433`
in a generated `collector-device-*` Compose project. Only this installation exposes
private management at the test NAS's named HTTPS origin; its HTTP and client TLS
listeners stay on loopback. No production proxy or tunnel is used.

The collection **Phone LAN test** contains **LAN test album**, two generated
30-second stereo FLAC/MP3 originals with tags, and a blue/green PNG cover. The
`phone` account starts as a non-administrator Viewer. Only synthetic media and
generated test credentials are used. The one-off operator controller and its
private handoff stay outside Git; credentials and TLS private keys are not recorded
here. The real import/preservation/analysis/artwork pipeline produced the content.

A separate short-lived ECDSA P-256 CA signed the fixture server certificate for
the selected NAS DNS name and private IPv4 address. The root lasts two days, the leaf one day;
the CA signing private key was destroyed after issuance. The downloadable profile
contains exactly one public root certificate, with no identity key, VPN or management
enrollment payload. The private operator handoff records its root certificate
fingerprint and exact test addresses; infrastructure details are omitted here.

An isolated private-IP HTTP helper serves only the public certificate
directory. The actual delivered profile bytes and Apple profile MIME were checked.
Python verified the leaf's CA chain and DNS name over TLS 1.3. Chrome 153.0.8010.37
then passed actual Viewer sign-in, playback/seek, exact original downloads,
persistent playback and mobile-layout checks. Its test-only browser TLS context was
preceded by independent CA/name/ServerId validation; this is not OS-trust or iOS
acceptance. Diagnostic screenshots remain in the operator's temporary artifacts.

Before/after comparison of the live Collector, Portainer and both tunnel container
IDs, image IDs, start times and running states matched. A fixture-only guardian
serves the public certificate until completion or six hours after preparation,
then stops/removes its Compose installation and generated data. The operator must
also remove the temporary certificate profile from the device after testing.

## Physical observations and remaining checks

The operator confirmed the certificate was trusted, the artwork appeared, and both
FLAC/MP3 tracks played and sought correctly in physical Safari. This used the
temporary public-root profile and Apple's [profile installation](https://support.apple.com/en-us/102400)
and [manual certificate trust](https://support.apple.com/en-us/102390) procedure.
The output device was not separately identified; no native-client claim follows.

After promoting the same non-administrator `phone` account from Viewer to Curator,
the operator used the real music editor to rename the first track to
**Safari permission check** and reported persistence after reload. An independent
owner API read confirmed that exact track title; the MP3 track retained its title.
All three saved originals still matched their frozen lengths and SHA-256 digests.
The account was returned to Viewer for the next physical check.

Viewer controls/denial, collection-grant revocation during listening and active
browser-session revocation still require physical observations. Internet-
disconnected LAN behavior remains pending until actually exercised; native
enrollment/output-device qualification is outside this Safari result.

## Independent server permissions and revocation assertions

A separate generated non-administrator account and collection exercised actual
commands without changing the phone account or its track edit:

- The same valid new-item write returned 403 as Viewer and succeeded as Curator.
  After demotion, an update returned 403 and the saved title was unchanged.
- A real original-media range returned 206 and exactly matched the selected bytes
  before that probe account's collection grant was removed. The existing session
  then received 403 for item, whole-media and ranged-media reads.
- Revoking the probe's current browser session returned 401 for subsequent identity
  and formerly authorized item reads.

All assertions passed with exit status 0 against the actual published runtime.
These API results do not substitute for the phone's visible denial, playback
termination or sign-in transition; those are recorded separately when observed.

## Newly observed numeric-IP startup failure

An initial configuration using a private IPv4 HTTPS URL as `authentication_origin`
completed account setup, but the published server
failed during startup before binding its listeners. The actual WebAuthn error was
`rp_id is not an effective_domain of rp_origin`, followed by invalid configuration.
Changing the authentication origin to the DNS name allowed startup and the above
preflight. This is a reproducible limitation of the currently documented numeric-IP
installation route, not a Safari trust failure. Repair the startup/setup contract
and validate its user-visible capability/guidance before claiming that route works.
