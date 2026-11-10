//! Isolated, bounded FFmpeg pipes for live playback; no archive or Job access.
use collector_app::{StreamCodec, StreamInput};
use collector_core::{Error, Result};
use std::{io, io::Read, path::Path, process::Stdio};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, ChildStdout, Command},
    sync::mpsc,
    task::JoinHandle,
};

pub struct LiveEncoder {
    child: Child,
    output: ChildStdout,
    input: Option<JoinHandle<io::Result<()>>>,
}
impl LiveEncoder {
    pub fn start(
        program: &Path,
        source: StreamInput,
        codec: StreamCodec,
        start_ms: u32,
    ) -> Result<Self> {
        let mut command = Command::new(program);
        #[cfg(unix)]
        command.process_group(0);
        command.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-xerror",
            "-max_alloc",
            "67108864",
            "-filter_threads",
            "1",
            "-threads",
            "1",
            "-protocol_whitelist",
            "pipe",
        ]);
        match &source {
            StreamInput::Saved(_) => {
                return Err(Error::Invalid("Saved audio needs no encoder".into()));
            }
            StreamInput::File(_, demuxer) => {
                command.args(["-f", demuxer]);
            }
            StreamInput::Pcm(reader, _) => {
                let format = reader.format();
                let raw = match format.bits {
                    16 => "s16le",
                    24 => "s24le",
                    _ => {
                        return Err(Error::Invalid(
                            "Unsupported PCM width for live audio".into(),
                        ));
                    }
                };
                command.args([
                    "-f",
                    raw,
                    "-ar",
                    &format.sample_rate.to_string(),
                    "-ac",
                    &format.channels.to_string(),
                ]);
            }
        }
        command.args(["-i", "pipe:0"]);
        if start_ms > 0 && matches!(source, StreamInput::File(..)) {
            command.args([
                "-ss",
                &format!("{}.{:03}", start_ms / 1000, start_ms % 1000),
            ]);
        }
        command.args([
            "-map",
            "0:a:0",
            "-map_metadata",
            "-1",
            "-map_chapters",
            "-1",
            "-threads",
            "1",
        ]);
        match codec {
            StreamCodec::Flac => {
                // Experimental mode is needed for >24 bit FLAC in newer FFmpeg.
                // 16/24 bit sources retain their native sample depth.
                command.args([
                    "-c:a",
                    "flac",
                    "-compression_level",
                    "2",
                    "-strict",
                    "experimental",
                    "-f",
                    "flac",
                ]);
            }
            StreamCodec::Opus(rate) => {
                command.args([
                    "-c:a",
                    "libopus",
                    "-b:a",
                    &format!("{rate}k"),
                    "-vbr",
                    "on",
                    "-page_duration",
                    "100000",
                    "-f",
                    "ogg",
                ]);
            }
        }
        let mut child = command
            .args(["-flush_packets", "1", "pipe:1"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| {
                Error::Unavailable("Live audio requires FFmpeg on the Collector server".into())
            })?;
        let mut stdin = child.stdin.take().expect("piped stdin");
        let output = child.stdout.take().expect("piped stdout");
        let (send, mut receive) = mpsc::channel(2);
        std::thread::Builder::new()
            .name("live-audio-source".into())
            .spawn(move || {
                if let Err(error) = feed(source, &send) {
                    let _ = send.blocking_send(Err(error));
                }
            })
            .map_err(|_| Error::Unavailable("Could not start the live audio reader".into()))?;
        let input = tokio::spawn(async move {
            while let Some(chunk) = receive.recv().await {
                stdin.write_all(&chunk?).await?;
            }
            stdin.shutdown().await
        });
        Ok(Self {
            child,
            output,
            input: Some(input),
        })
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(input) = &mut self.input {
            tokio::select! {
                biased;
                result = input => {
                    self.input = None;
                    result.map_err(io::Error::other)??;
                }
                result = self.output.read(buf) => return result,
            }
        }
        self.output.read(buf).await
    }

    pub async fn stop(&mut self) {
        if let Some(input) = self.input.take() {
            input.abort();
        }
        self.kill_group();
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
    }

    fn kill_group(&self) {
        #[cfg(unix)]
        if let Some(pid) = self.child.id() {
            super::conversion::kill_process_group(pid);
        }
    }

    pub async fn finish(&mut self) -> io::Result<()> {
        let status = self.child.wait().await?;
        if let Some(input) = &mut self.input {
            let result = input.await.map_err(io::Error::other)?;
            self.input = None;
            result?;
        }
        if !status.success() {
            return Err(io::Error::other("live audio encoding failed"));
        }
        Ok(())
    }
}

impl Drop for LiveEncoder {
    fn drop(&mut self) {
        if let Some(input) = &self.input {
            input.abort();
        }
        self.kill_group();
        // kill_on_drop reaps the leader if the supervising task itself is dropped.
    }
}

fn feed(source: StreamInput, send: &mpsc::Sender<io::Result<Vec<u8>>>) -> io::Result<()> {
    match source {
        StreamInput::Saved(_) => unreachable!(),
        StreamInput::File(mut media, _) => {
            let mut remaining = media.length;
            while remaining > 0 && !send.is_closed() {
                let mut chunk = vec![0; remaining.min(32768) as usize];
                media.reader.read_exact(&mut chunk)?;
                remaining -= chunk.len() as u64;
                if send.blocking_send(Ok(chunk)).is_err() {
                    break;
                }
            }
        }
        StreamInput::Pcm(mut reader, mut remaining) => {
            let frame_bytes = reader.format().frame_bytes();
            while remaining > 0 && !send.is_closed() {
                let wanted = remaining.min((32768 / frame_bytes) as u64) as usize;
                let mut chunk = vec![0; wanted * frame_bytes];
                let frames = reader.read_pcm(&mut chunk)?;
                if frames == 0 || frames > wanted {
                    return Err(io::Error::other("unexpected live PCM length"));
                }
                remaining -= frames as u64;
                chunk.truncate(frames * frame_bytes);
                if send.blocking_send(Ok(chunk)).is_err() {
                    break;
                }
            }
        }
    }
    Ok(())
}
