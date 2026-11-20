# Junk Collector development

Read [architecture](docs/architecture.md) before changing a subsystem.

## Invariants

- Keep the layering: domain/core code has no HTTP, filesystem, database, GUI or
  device dependencies; application services own policy; adapters own mechanisms;
  HTTP handlers call application services, not the Store.
- Never lose user data. Preserve exact originals, physical inventory facts,
  manual metadata/artwork choices, accepted provider sources, recorded evidence
  and unknown module data. Changing a stored format needs a conversion or a clear
  refusal before writing — never a silent reset. Identical repeated lookups do
  not need to be kept forever.
- Keep integrity hashes, structural checks, provisional metadata, MusicBrainz
  identity and catalog/AccurateRip verification as separate claims. Never claim
  more coverage than was computed.
- Public vs private access comes from the listener, never source IP or forwarded
  headers. Roles are enforced regardless of exposure.
- Browsing, polling and job admission must not hash or read full payloads; do
  byte validation in cancellable background work.
- Shared parsing and audio algorithms belong in narrow `junk-libs` crates (or
  `crates/junk-libs-pcm`), not in application crates. Check other consumers when
  changing them.
- Don't ship a mock that looks like a working feature.
- Before a stable release, interfaces and storage formats may change together
  without compatibility shims, as long as existing data is preserved.

## Build and test

```sh
npm --prefix web ci && npm --prefix web run build   # rebuild embedded web assets
cargo build -p collector-server
python3 scripts/check_architecture.py               # layering + doc links
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 -m unittest discover -s scripts -p 'test_*.py'
npm --prefix web run check
```

Rebuild the server after changing web assets; they are embedded in the binary.
After changing an API route or DTO run `python3 scripts/api_contract.py --write`.
For UI changes, check the real screen, not only text assertions. Test behavior,
not incidental wording or layout. See [testing](docs/testing.md).

## Docs

Use generic host names, placeholder paths and example domains. Don't write dates,
ticket codes or delivery logs into docs; Git keeps history.
