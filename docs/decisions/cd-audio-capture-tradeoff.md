# Audio-CD capture: optional Q refinement

Status: proposed, not yet reviewed by the owner.

For ordinary music CDs, Collector keeps the complete raw capture and relies on
AccurateRip to check the audio, accepting Q-subchannel errors rather than requiring
perfect subchannel recovery.

## Decision

- Keep the complete redumper package: raw masters, SUBCODE, STATE, TOC and logs.
  Unresolved Q errors stay visible; subchannels are never discarded or synthesized.
- `scripts/rip-cd.sh` never refines. When the first pass has SCSI/C2 errors it
  says to check the disc and prints a manual refine command; Q-only errors aren't
  flagged. Refining a damaged disc can take hours and can leave the drive reading
  slowly until it is reconnected, so cleaning the disc first and refining by hand
  is the owner's choice. Extra Q recovery is likewise an optional manual step,
  never a gate on importing or listening.
- Audio is checked with Collector's AccurateRip workflow, within its actual
  coverage. A whole-disc claim needs every expected track and a justified offset;
  one matching track is not enough.
- An AccurateRip match doesn't prove the subchannels, index/control data or
  samples outside coverage are correct. A missing reference, partial coverage or
  mismatch doesn't by itself prove damaged audio.
- Persistent Q-only errors don't reopen recovery work. Reassess when there are
  audio read errors, suspicious track/index boundaries or control flags, audible
  problems, or subchannel content that matters (such as CD+G).

## Why

Q carries position and control data, not audio. It has a CRC but isn't covered by
the audio's CIRC error correction, so occasional bad Q packets can coexist with
correctly recovered audio. Normal music-ripping workflows keep lossless audio plus
a log; this project keeps more than that. Mastering-plant subcode error averages
are not a standard for consumer-drive captures.

## Manual Q refinement

Find the drive (`diskutil list`, then `diskutil info /dev/diskN` on macOS), keep
the same disc inserted, and run against the existing capture folder (fish syntax):

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

This updates the capture in place. Drop `--speed=8` to use the drive's default;
the drive may not honor the requested speed. Under **correction statistics**, Q
counts sectors repaired in that pass, not errors remaining. Stop when further
passes aren't worth it.

## References

- [ECMA-130](https://ecma-international.org/wp-content/uploads/ECMA-130_2nd_edition_june_1996.pdf) (sections 17, 18, 22.3: Q CRC vs. CIRC)
- [redumper](https://github.com/superg/redumper)
- [Hydrogenaudio: EAC lossless backup](https://wiki.hydrogenaudio.org/index.php?title=EAC_Lossless_Backup)
