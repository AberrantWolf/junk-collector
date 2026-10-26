# Home-lab release capability and evidence checklist

Reviewed 2026-10-21 for beta.2 correction. This is a capability/evidence checklist;
[remaining work](remaining-work.md) remains the only assignment queue. Beta.2
retains explicit supported-scope limits. No stable API/storage/upgrade promise is
introduced by this review.

| Capability | Accepted evidence | Scope or remaining limit |
| --- | --- | --- |
| Originals, manual authority, meaningful evidence and unknown records | Workspace recovery/reindex tests, [retention audit](retention-audit.md), [portable transfer](validation/archive-export-2026-10-21.md) | Full portable transfer is distinct from operational backup; no generic archive pruning |
| Web setup, intake, metadata/artwork, listening and lists | [Alpha.3 source/Chrome acceptance](validation/container-publication-2026-10-21.md), [digital purchases](validation/digital-purchases-2026-10-19.md) | Named supported CD layouts and FLAC/MP3 originals; explicit codec/format limits |
| Mobile roles, playback/seek and revocation | [Physical Safari acceptance](validation/lan-device-preparation-2026-10-20.md) | Named iPhone/Safari; no every-device claim |
| Native streaming, explicit downloads and offline originals/artwork/text/search | [Trash Amp purchase acceptance](validation/digital-purchases-2026-10-19.md) | [Bounded command/actual-read/range-fault/trust recovery](validation/native-failure-recovery-2026-10-21.md) also passes; remaining system-trust/device-loss/native UI/account-switching cases remain partial under HL-04/08 |
| Native Linux images, anonymous pulls and Portainer updates | [Alpha.3 publication](validation/container-publication-2026-10-21.md); [Beta.2 publication and NAS acceptance](validation/container-beta2-publication-2026-10-21.md) | Native AMD64/ARM64 and isolated Synology path; live production was not updated |
| Update recovery, permission/capacity/preparation failures | [NAS operations](validation/nas-operations-2026-10-20.md), [local recovery points](validation/snapshot-recovery-2026-10-20.md), HL-05C–G in the queue | Named bounded failures; no arbitrary power-loss/storage certification |
| Public/private surfaces, authentication and media authorization | [Focused ingress review](validation/ingress-review-2026-10-21.md), existing HTTP/access regressions | Reuses actual-origin/operator evidence; bounded single-process controls, no enterprise certification |
| FLAC/Opus preparation | [Independent real-tool audio oracles](validation/audio-oracles-2026-10-20.md) | Named fixture/tool/client; no implied gapless/device-bit-perfect/whole-disc catalog verification |
| Catalog/audio identity claims | [Supported game matrix](game-identification.md), [audio matrix](audio-preparation.md) | Additional owned format/AccurateRip references remain required for broader claims |
| PDF/text document attachments | [Beta.2 feature acceptance](validation/documents-2026-10-21.md) | Original-preserving uploads/downloads; native browser PDF viewer and bounded text preview; no server PDF rendering or all-browser claim |
| Optional backup-age reminder | [Beta.2 feature acceptance](validation/backup-reminder-2026-10-21.md) | Recorded validation/external reports only; no scheduler, background media scan or assertion that a backup still exists |
| Game-disc CD audio | Beta.2 packages source with owned/browser/native streaming and offline acceptance | Exact shared/Collector/native pins qualified; broader physical/raw-format coverage and native application packaging remain separate |

Per-release source coverage follows [the change-based testing policy](testing.md):
text-only edits do not trigger unit tests; beta selects affected checks and
RC/stable requires full functional qualification. Current-stable toolchain selection,
embedded-asset rebuild, relevant actual UI journeys and digest-pinned packaging
remain required. Deployment executables are release builds; container qualification
uses the exact images without an extra discarded standalone release compilation. Reuse
unaffected acceptance instead of repeating the whole deployment/device matrix.
A supported release must select a finite client/format scope, link its evidence,
retain visible limitations and describe complete matching-point recovery. The
remaining native/device and owned-reference cases do not become passed because
an image, build or synthetic fixture succeeds.
