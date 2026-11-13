# Large music package analysis — 2026-11-13

Production import records showed two published packages with 260 and 308 files
whose analysis was skipped. The serialized declarations exceeded the 64 KiB
processing selection limit before a Job could be created. The original files
were published.

The source fix submits playback choices by inventory position. The store
regression imported 310 files with long names and a reviewed playback choice.
Analysis completed, all 310 originals were retained, and the album resolved to
the reviewed original. The Collector module suite, focused float WAV and
retained-analysis regressions, and the Chrome 156 inbox intake journey passed.
The browser journey covered 31 inbox imports, progress, and restart.

These checks used local fixtures. This change was not deployed or tested
against the two production packages.
