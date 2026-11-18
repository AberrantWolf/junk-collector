# Digital album track review and AcoustID contributions

The Chrome digital-matching journey used real WAV and MP3 originals, a local
Collector server, and offline provider fixtures. The editor kept the recognized
track, filled the unique tentative assignment, and required confirmation.
Changing a selection invalidated confirmation. Desktop and mobile review showed
the paired file and release track, with a separate 28-track fixture for the
larger preview.

The fixture received one POST carrying the unmatched file's computed
fingerprint, full duration and reviewed recording ID. It also verified the two
API keys and the exact outbound field set. Retrying recovered the original
submission ID without another POST. A stale review was rejected and contribution
left local metadata unchanged. Provider and store checks covered incomplete
acknowledgments and replay after server interruption.

This validates Collector's workflow with an offline provider. Public AcoustID
submission, subsequent recognition and deployment were outside this check.
See the [earlier matching review](digital-matching-and-review-2026-11-03.md)
for the preceding workflow and its qualification limits.
