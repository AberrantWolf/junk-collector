# Audio-CD capture: optional Q refinement and independent audio checks

Accepted user direction, 2026-10-27. This applies the
[home-lab scope](home-lab-scope.md) to ordinary music-CD capture. It records an
accepted preservation tradeoff, not a new ripping subsystem or a claim that
incomplete captures are perfect disc archives.

Status: Accepted (original direction 2026-10-27; normative wording formalized
2026-11-03). See [ADR 0006](0006-source-bound-evidence-and-coverage.md) for claim
coverage and [ADR 0008](0008-meaningful-provenance-and-bounded-lookup-history.md)
for preservation. The specialized rules below apply to ordinary music-CD capture.

## Decision

- The complete captured redumper package must be preserved, including raw masters,
  SUBCODE, STATE, TOC and other sidecars/logs. Unresolved Q errors must remain
  visible; subchannels must not be discarded or synthesized to imply completeness.
- Automatic refinement in `scripts/rip-cd.sh` must remain conditional on initial
  SCSI/C2 errors.
  Q-only errors must not trigger normal automatic refinement. Additional Q recovery
  may be an explicit operator action, but must not become an ordinary capture,
  import or listening gate or a requirement to buy another drive.
- Independent audio assessment under this tradeoff must use Collector's existing
  AccurateRip workflow. Results must be interpreted within their actual source, reference,
  algorithm/version, track and offset-search coverage. Whole-disc claims must meet
  complete expected-track and justified-offset conditions; one matching track
  must not establish a whole-disc claim.
- An audio match must not be presented as proof of complete/correct subchannels,
  all index/control information or samples outside coverage. Missing references,
  partial coverage, ambiguity or a mismatch must not alone be presented as proof
  of damaged audio. Results must remain available for investigation. A clean
  SCSI/C2 summary must not be presented as proof of complete subchannels, all
  index/control information or samples outside the reported coverage.
- Persistent Q-only counts must not by themselves reopen mandatory recovery work.
  Concrete conflicting evidence must prompt reassessment: unresolved audio-read
  errors, suspicious track/index boundaries or control flags, audible problems,
  or required subchannel content such as CD+G. Existing structural validation and
  unsupported-format restrictions must still apply.

## Why this tradeoff is acceptable

Q carries control/position information rather than PCM audio. Its CRC detects
errors, but Q does not receive the audio's CIRC error correction. Occasional bad
Q packets can coexist with correctly recovered audio, and repeated/predictable
position information can keep ordinary playback working. Music-ripping workflows
commonly retain lossless audio, a log and optionally a CUE sheet instead of every
raw subchannel packet. The project retains the fuller raw package while accepting
documented subchannel imperfections for the user's hobbyist music archive.

This is an explicit scope choice, not a numerical manufacturing certification.
Historical StageTech EC2 mastering guidance gives instrument-dependent acceptable
bad-subcode averages; it is not a universal pressed-CD Q-error allowance or a
calibrated comparison with a consumer-drive capture. The commonly quoted BLER
limit concerns C1-input blocks, not the Q count. Whole-disc averages do not
establish correctness of track boundaries or control data.

See the [operator observation](../validation/raw-cd-capture-2026-10-25.md#accepted-q-only-tradeoff--2026-10-27)
for the owned-disc example that prompted this decision. It includes no claimed
AccurateRip result.

## Optional manual Q refinement (fish)

Find the intended drive with `diskutil list` on macOS; confirm a candidate with
`diskutil info /dev/diskN`. Keep the same disc inserted. Replace these example
values with the existing capture folder and the intended current drive:

```fish
set q_drive diskN
set q_dump "/path/to/existing/dump"
redumper refine \
  --drive="$q_drive" \
  --image-path="$q_dump" \
  --image-name=disc \
  --speed=8 \
  --retries=100 \
  --refine-subchannel \
  --verbose
```

This updates that capture in place. Omit `--speed=8` to request the drive's
default optimal speed. A requested speed, including the speed printed in
redumper's drive information, is not a measurement that the drive obeyed it.
Under **correction statistics**, Q counts sectors repaired during that pass,
not errors remaining; zero means no additional Q corrections. Keep the log
and stop repeating passes when the operator judges the remaining benefit too low.

## Supporting references

- [CD format and Q CRC/CIRC separation (ECMA-130, sections 17, 18 and 22.3)](https://ecma-international.org/wp-content/uploads/ECMA-130_2nd_edition_june_1996.pdf).
- [Redumper raw subchannel preservation and refinement](https://github.com/superg/redumper),
  [refinement implementation and correction statistics](https://github.com/superg/redumper/blob/main/cd/cd_dump.ixx).
- [Music-ripping workflow (Hydrogenaudio)](https://wiki.hydrogenaudio.org/index.php?title=EAC_Lossless_Backup).
- [StageTech EC2 manual, pages 71 and 85 (archived manufacturer guidance)](https://manualzilla.com/doc/6864989/user-s-manual-v-1.08-us---stagetech-developments).
