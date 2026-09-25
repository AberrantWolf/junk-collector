# FX-04 growing-list audit

Checked 2026-09-24. Counts are expected product cardinalities, not release-scale performance claims. Pages are scoped and authorized by the server unless stated otherwise.

| Surface | Expected growth | Query and visible bound | Finding a later record |
|---|---:|---|---|
| Collection browse/search | Thousands of entries | Server search and keyset pages of 50 | Search or Load more; direct item URL rechecks authorization |
| Guided import destination | Thousands of entries | Search opens on demand; server pages of 50, obsolete searches cancelled | Search and More results; selection remains visible outside its result page and revision is rechecked on submit |
| Manual import review destination | Thousands of entries | Same destination chooser | Search and More results; import review still checks current item revision |
| Work & history | Thousands of imports and Jobs | Server search and keyset pages of 25; older loaded rows are retained locally without individual refetches each poll | Search, status/date filters and Load older work; import detail has an authorized direct link and a separate view |
| Execution history within one work item | Potentially many attempts | Server pages of 25 | Load older executions |
| Guided current-import progress | One accepted package at a time in the panel | One `/ingests/{id}` read on each poll; no catalog or global history load | Open Work & history for other imports |
| Catalog snapshots | Potentially many per collection | Server pages of 50 | More catalog snapshots |
| Preparation batches | Potentially many per item | Existing server read returns the 50 most recent | UI states the 50-batch bound and points to Work & history for older Jobs; a dedicated paged batch view remains a broader history task |
| Configured inbox packages | Potentially many directories | Inbox labels load without package enumeration; authorized server package search returns at most 100 per page (50 in the UI), scanning direct directories with bounded response memory | Search or More packages; selected packages remain visible outside the result page |
| Accounts and collection grants | Potentially many users | Existing authorized endpoints return complete lists | Access-specific search/pagination belongs to FX-06 collection settings and account recovery |
| Provider candidates and artwork for one item | Bounded per provider observation and one item | One item-bound metadata review; provider response parser has explicit bounds | Advanced Evidence shows current candidates and selectable history, rather than a global selector |

The access surfaces have no hidden first-page cutoff, but their complete-list loading needs FX-06's access-specific search and pagination. Inbox package enumeration still scans one direct directory per query; it does not load every package into the browser. FX-07 should recheck all surfaces after FX-06's access changes. This audit does not qualify general large-scale indexing.
