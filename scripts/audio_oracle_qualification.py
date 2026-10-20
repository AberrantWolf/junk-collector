"""Independent real-tool oracles for the generated CD-audio qualification fixture."""
from array import array
import hashlib
import io
import json
import math
import struct
import sys
import wave


class AudioOracleQualification:
    def __init__(self, run):
        self.run = run
        self.results = []

    @staticmethod
    def source_pcm(frames):
        # Separate channel frequencies expose silence, channel swaps, gain errors
        # and timing mistakes. This formula is independent of the product decoder.
        return b"".join(struct.pack("<hh",
            round(8000 * math.sin(2 * math.pi * 997 * n / 44100) + 2000 * math.sin(2 * math.pi * 443 * n / 44100)),
            round(7500 * math.sin(2 * math.pi * 1499 * n / 44100) + 2300 * math.sin(2 * math.pi * 661 * n / 44100)))
            for n in range(frames))

    def tool(self, executable, *arguments, **kwargs):
        return self.run("run", "--rm", "-T", "--entrypoint", executable, "cli", *arguments,
                        capture_output=True, **kwargs)

    @staticmethod
    def samples(pcm):
        values = array("h")
        values.frombytes(pcm)
        if sys.byteorder != "little":
            values.byteswap()
        return values

    def decode(self, encoded):
        # A seekable WAV lets opusdec write the actual final length rather than a
        # streaming placeholder. Only this generated root is used for scratch.
        return self.tool("sh", "-eu", "-c",
            "mkdir -p /data/state/qualification-audio; "
            "cat > /data/state/qualification-audio/input.opus; "
            "opusdec --quiet --rate 44100 --force-wav /data/state/qualification-audio/input.opus /data/state/qualification-audio/output.wav; "
            "cat /data/state/qualification-audio/output.wav; "
            "rm /data/state/qualification-audio/input.opus /data/state/qualification-audio/output.wav",
            input=encoded).stdout

    @staticmethod
    def signal(decoded, source):
        with wave.open(io.BytesIO(decoded), "rb") as reader:
            assert (reader.getnchannels(), reader.getsampwidth(), reader.getframerate()) == (2, 2, 44100)
            assert reader.getnframes() == len(source) // 4, "Decoded Opus length differs from the frozen CD range"
            pcm = reader.readframes(reader.getnframes())
        actual, expected = AudioOracleQualification.samples(pcm), AudioOracleQualification.samples(source)
        assert len(actual) == len(expected)
        outcomes = []
        # Omit 100 ms at each end from signal correlation only; total decoded
        # length above includes every frame. No alignment search hides a delay.
        for channel in (0, 1):
            left, right = actual[8820 + channel:-8820:2], expected[8820 + channel:-8820:2]
            power = sum(value * value for value in left)
            reference = sum(value * value for value in right)
            assert power > 0 and reference > 0
            correlation = sum(a * b for a, b in zip(left, right)) / math.sqrt(power * reference)
            gain = math.sqrt(power / reference)
            assert correlation > 0.98 and 0.8 < gain < 1.2, (channel, correlation, gain)
            outcomes.append(dict(correlation=round(correlation, 6), gain=round(gain, 6)))
        return outcomes

    @staticmethod
    def row(request, cookie, item, codec, bitrate=None):
        rows = request(f"/items/{item}/music-tracks", cookie=cookie)
        selected = [row for row in rows if (row.get("profile") or {}).get("codec") == codec
                    and (bitrate is None or row["profile"]["bitrate_kbps"] == bitrate)]
        assert len(selected) == 1 and selected[0]["state"]["status"] == "available", rows
        return selected[0]

    @staticmethod
    def bytes(request, cookie, item, row):
        state = row["state"]
        status, headers, content = request(f"/items/{item}/representations/{state['representation_id']}/{state['file_index']}", cookie=cookie, raw=True)
        assert status == 200 and len(content) == state["length"]
        assert hashlib.sha256(content).hexdigest() == state["digest"]["hex"]
        status, _, partial = request(f"/items/{item}/representations/{state['representation_id']}/{state['file_index']}", cookie=cookie, raw=True,
                                    headers={"Range": "bytes=4-15"})
        assert status == 206 and partial == content[4:16]
        return content

    def exercise(self, cookie, seed, request, verify, handoff, source, flac_item):
        for executable in ("flac", "opusenc", "opusdec"):
            result = self.tool(executable, "--version")
            report = (result.stdout or result.stderr).decode().strip()
            print("Independent tool: " + executable + " " + report, flush=True)
        flac = self.row(request, cookie, flac_item, "flac")
        encoded = self.bytes(request, cookie, flac_item, flac)
        decoded = self.tool("flac", "--decode", "--stdout", "--force-raw-format", "--endian=little", "--sign=signed", "-", input=encoded).stdout
        assert decoded == source, "Independent FLAC decoder did not recover every expected integer sample"
        print("Independent libFLAC decode equals the complete non-silent stereo source PCM", flush=True)
        self.results.append((flac_item, "flac", None, flac["state"]))
        for bitrate in (96, 128, 192):
            item = seed("Opus " + str(bitrate) + " kbps")
            handoff(item, scenario="prepare-opus-" + str(bitrate), bitrate=bitrate)
            row = self.row(request, cookie, item, "opus", bitrate)
            # The normal player's fallback really had to decode Opus, rather than
            # quietly satisfying playback with a preferred FLAC of this source.
            rows = request(f"/items/{item}/music-tracks", cookie=cookie)
            assert not any((value.get("profile") or {}).get("codec") == "flac" and value["state"]["status"] == "available" for value in rows)
            encoded = self.bytes(request, cookie, item, row)
            outcomes = self.signal(self.decode(encoded), source)
            verify(cookie, item)
            self.results.append((item, "opus", bitrate, row["state"]))
            print("Opus oracle: " + json.dumps(dict(bitrate=bitrate, bytes=len(encoded), frames=len(source)//4, channels=outcomes)), flush=True)

    def verify_restored(self, cookie, request, verify, source, handoff):
        for item, codec, bitrate, previous in self.results:
            verify(cookie, item)
            row = self.row(request, cookie, item, codec, bitrate)
            assert row["state"] == previous, "Restore changed immutable prepared audio identity"
            encoded = self.bytes(request, cookie, item, row)
            if codec == "opus":
                self.signal(self.decode(encoded), source)
                handoff(item, scenario="restored-opus-" + str(bitrate), bitrate=bitrate)
        print("All real-tool audio identities/bytes/Opus decoder oracles survived full restore", flush=True)
