# Actual NAS authentication thresholds — HL-05G, 2026-10-20

## Scope

A separate generated Synology DS1522+ / Btrfs installation passed finite actual-TCP
authentication account, peer and global limit checks, restart reset and full browser/
backup/restore acceptance. Docker 24.0.2, Compose 2.20.1 and Python 3.8.15 used the
published alpha.2 image pinned to:

```text
ghcr.io/aberrantwolf/junk-collector@sha256:f86f46d2c8870f8d647fc9225ce2cfe7c4319521a29c89897b419abb77629433
```

Its source remains `0f0e0f52fedc239ecdb180436583e69b7ce13a75`. This qualifies the
shipped threshold/direct-peer behavior. The separate [identity-retention repair](rate-limit-retention-2026-10-20.md)
is newer source; this run does not imply the published image contains that fix.

`scripts/authentication_rate_qualification.py` makes real HTTP connections with
Linux loopback **source addresses**, not mocked router extensions or forwarded
headers. Public and private loopback listeners share the same process. All attempts
are confined to the fresh test installation and finite phases under 45 seconds,
with normal fixture restarts between cases. It creates no production account,
touches no real media and runs no sustained load or resource-exhaustion attack.

## Acceptance

The whole harness and real browser driver completed with exit status 0:

| Case | Independent actual-TCP check | Result |
| --- | --- | --- |
| Account | Same nonexistent account, ten distinct source IPs; eleventh from another IP | First ten reach ordinary 401; eleventh returns typed 429/Retry-After 60. An unrelated account still reaches ordinary 401. |
| Direct peer | One real source, one valid login plus 29 distinct failed accounts across both listeners, rotating forwarding headers | At 30 admitted attempts, 32 more aliases stay throttled. Valid owner login from another real source succeeds. |
| Global | Four real source IPs, 25 distinct failed accounts each, split across both listeners | All 100 reach ordinary 401 while staying below peer/account ceilings. Another 100 fresh account/source combinations return typed 429/Retry-After 60. |
| Existing session | Original session used while the global limit is exhausted | `/me` still succeeds for the same owner. |
| Restart | Stop/reopen the same installation after each exhausted case | Original session/account survives; correct password login works again. Limits are transient, not permanent account locks. |

The normal journey first preserved generated CUE/BIN/hidden originals, retained a
partial upload through container recreation and prepared real FLAC. Missing provider
configuration remained visibly unavailable. Chrome 155.0.8059.85 then passed real
playback/seek, byte/SHA-256 original downloads, persistent player and 390px layout
before and after stopped full backup/validation/restore. Session, owner, ServerId,
configuration/TLS and exact originals/prepared audio survived. Clean stops took
0.22–0.33s. The browser independently validates PEM/name/ServerId, then uses an
isolated TLS fixture context; it installs no OS or phone trust.

Generated containers, data/backup roots, private handoffs and the source-transfer
directory were absent after cleanup. Live Collector, Portainer and both tunnels
matched their original IDs, image IDs, start times and running state. No production
installation, credentials, tunnel, OS trust or real media was changed.

Python 3.8 syntax/actual NAS execution, architecture/document/fixture checks,
container-tools and release-policy tests passed. Runtime Rust and product assets
are unchanged by this qualification; the source repair's 443-test/Clippy/full-Chrome
gates remain in its separate record.

## Reproduction and limits

Use the [HL-05C isolated reproduction](nas-preparation-recovery-2026-10-20.md#reproduction-and-checks)
with `--authentication-limits` instead of `--preparation-faults`. Fresh inbox and
browser fixtures are required; update/snapshot/Portainer/other fault modes are
rejected. Browser handoffs are the ordinary filename and `-restored`. Delete local
credential copies and mark the remote `.done` file only on actual browser success.

These are multiple genuine TCP source addresses on one Linux host, not multiple
physical clients, a public proxy/tunnel, NAT behavior or distributed traffic. They
do not qualify enrollment/recovery floods, slow/oversized media, CPU/memory pressure,
duration/throughput guarantees, live deployment exposure or public authenticator
behavior. The newer limiter memory-bound source evidence remains separate.
