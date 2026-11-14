//! Normal authenticated HTTP live audio. Never falls through a private audio policy.
use super::*;
use axum::http::Method;
use collector_app::{MusicStreamRequest, StreamCodec, StreamCoordinate, StreamInput};
use collector_providers::LiveEncoder;
use std::{
    io::{self, Read, Seek, SeekFrom},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::mpsc;

#[derive(Default)]
pub(crate) struct LiveSlots(Mutex<std::collections::HashMap<String, usize>>);
struct Lease(Arc<LiveSlots>, String);
impl LiveSlots {
    fn acquire(self: &Arc<Self>, client: String) -> collector_core::Result<Lease> {
        let mut slots = self
            .0
            .lock()
            .map_err(|_| Error::Unavailable("Live audio admission unavailable".into()))?;
        // One normal playback request plus brief overlap on track switch/seek.
        if slots.values().sum::<usize>() >= 8 || slots.get(&client).copied().unwrap_or(0) >= 2 {
            return Err(Error::RateLimited(1));
        }
        *slots.entry(client.clone()).or_default() += 1;
        Ok(Lease(self.clone(), client))
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if let Ok(mut slots) = self.0.0.lock()
            && let Some(count) = slots.get_mut(&self.1)
        {
            *count -= 1;
            if *count == 0 {
                slots.remove(&self.1);
            }
        }
    }
}

// Length-delimited CD WAV reads still traverse the original package.
struct AdmittedDiscReader {
    reader: Box<dyn collector_app::MediaReader>,
    _lease: Lease,
}

impl Read for AdmittedDiscReader {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.reader.read(bytes)
    }
}

impl Seek for AdmittedDiscReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.reader.seek(position)
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
enum PlaybackMode {
    Original,
    HighBandwidth,
    LowBandwidth,
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub(crate) struct StreamQuery {
    /// Select a playback mode. Defaults to High Bandwidth.
    #[param(inline)]
    mode: Option<PlaybackMode>,
    /// Low Bandwidth Opus target in kbps, 6–256. Defaults to 128.
    #[param(minimum = 6, maximum = 256)]
    bitrate: Option<u16>,
    /// Track number when playing a source-bound track.
    track: Option<u16>,
    /// File index when playing a directly queued original file.
    file: Option<usize>,
    /// Start position in milliseconds for a live stream.
    start_ms: Option<u32>,
}

fn allowed(state: &AppState) -> collector_core::Result<()> {
    if !state.direct_media
        && !state
            .service
            .audio_connection_policy(state.remote_media_allowed)?
            .remote_media_allowed
    {
        return Err(Error::Unavailable(
            "Playback modes need normal authenticated audio or a direct Collector listener".into(),
        ));
    }
    Ok(())
}

#[utoipa::path(get, path = "/api/v1/items/{id}/music-stream/{source}", params(("id"=ItemId,Path),("source"=RepresentationId,Path),StreamQuery), responses((status=200,content(("audio/flac"),("audio/ogg"),("audio/wav"),("audio/mpeg"))),(status=206),(status=400),(status=401),(status=403),(status=404),(status=429),(status=503)))]
pub(crate) async fn music_stream(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path((id, source)): Path<(ItemId, RepresentationId)>,
    Query(query): Query<StreamQuery>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers).await?;
    allowed(&state)?;
    let codec = match (
        query.mode.unwrap_or(PlaybackMode::HighBandwidth),
        query.bitrate,
    ) {
        (PlaybackMode::Original, None) => StreamCodec::Original,
        (PlaybackMode::HighBandwidth, None) => StreamCodec::Flac,
        (PlaybackMode::LowBandwidth, bitrate) => {
            StreamCodec::Opus(bitrate.unwrap_or(128)).validate()?
        }
        _ => {
            return Err(
                Error::Invalid("Bitrate applies only to Low Bandwidth playback".into()).into(),
            );
        }
    };
    let coordinate = match (query.track, query.file) {
        (Some(track), None) => StreamCoordinate::Track(track),
        (None, Some(file)) => StreamCoordinate::File(file),
        _ => return Err(Error::Invalid("Choose exactly one track or original file".into()).into()),
    };
    let start_ms = query.start_ms.unwrap_or(0);
    // Bound concurrent source opens, including HEAD; passthrough releases this
    // lease before serving bytes and consumes no transcoding slot.
    let user = actor.user_id;
    let credential = browser_session_cookie(&headers)
        .or_else(|| {
            headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
        })
        .ok_or(Error::Unauthenticated)?;
    let lease = state
        .live_audio_slots
        .acquire(Digest::sha256(credential.as_bytes()).hex)?;
    let service = state.service.clone();
    let cancelled = Arc::new(AtomicBool::new(false));
    let _cancel = CancelOnDrop(cancelled.clone());
    let request = MusicStreamRequest {
        coordinate,
        codec,
        start_ms,
    };
    let ((input, access), lease) = tokio::time::timeout(
        Duration::from_secs(15),
        blocking(move || {
            let checkpoint = || {
                if cancelled.load(Ordering::Acquire) {
                    Err(Error::JobCancelled)
                } else {
                    Ok(())
                }
            };
            let input = service.music_stream(&actor, id, source, request, &checkpoint)?;
            checkpoint()?;
            service.recheck_stream(&actor, input.1)?;
            Ok((input, lease))
        }),
    )
    .await
    .map_err(|_| Error::Unavailable("Opening the audio source timed out".into()))??;
    allowed(&state)?;
    let check_path = HeaderValue::from_str(&access.playback_check_path(query.track))
        .map_err(|e| Error::Storage(e.to_string()))?;
    if let StreamInput::Saved(mut saved) = input {
        if access.is_disc() {
            saved.reader = Box::new(AdmittedDiscReader {
                reader: saved.reader,
                _lease: lease,
            });
        } else {
            drop(lease);
        }
        let mut response = file_response(saved, headers).await?;
        response
            .headers_mut()
            .insert("x-collector-playback-check", check_path);
        return Ok(response);
    }
    let builder = Response::builder()
        .header(
            header::CONTENT_TYPE,
            codec.media_type().ok_or_else(|| {
                Error::Invalid("Original playback requires a playable source".into())
            })?,
        )
        .header(header::ACCEPT_RANGES, "none")
        .header(header::CACHE_CONTROL, "private, no-store")
        .header("x-accel-buffering", "no")
        .header("x-collector-playback-check", check_path);
    if method == Method::HEAD {
        return builder
            .body(Body::empty())
            .map_err(|e| Error::Storage(e.to_string()).into());
    }
    let mut encoder = LiveEncoder::start(&state.live_audio_ffmpeg, input, codec, start_ms)?;
    let (send, mut receive) = mpsc::channel::<io::Result<axum::body::Bytes>>(2);
    tokio::spawn(async move {
        let _lease = lease;
        let recheck = async {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let state = state.clone();
                let headers = headers.clone();
                let result = tokio::task::spawn_blocking(move || -> ApiResult<()> {
                    let current = crate::authenticate_request(&state.service, &headers)?.principal;
                    if current.user_id != user {
                        return Err(Error::Unauthenticated.into());
                    }
                    allowed(&state)?;
                    state.service.recheck_stream(&current, access)?;
                    Ok(())
                })
                .await;
                if !matches!(result, Ok(Ok(()))) {
                    return Err(io::Error::other("live audio access ended"));
                }
            }
        };
        let stream = async {
            loop {
                let mut buf = vec![0; 32768];
                let read =
                    tokio::time::timeout(Duration::from_secs(30), encoder.read(&mut buf)).await??;
                if read == 0 {
                    return tokio::time::timeout(Duration::from_secs(5), encoder.finish()).await?;
                }
                buf.truncate(read);
                tokio::time::timeout(Duration::from_secs(30), send.send(Ok(buf.into())))
                    .await?
                    .map_err(io::Error::other)?;
            }
        };
        let result = tokio::select! {
            _ = send.closed() => { encoder.stop().await; return; },
            result = recheck => result,
            result = stream => result,
            _ = tokio::time::sleep(Duration::from_secs(6 * 3600)) => Err(io::Error::other("live audio time limit")),
        };
        if let Err(error) = result {
            encoder.stop().await;
            tracing::warn!(%error, "live audio ended before completion");
            let _ = tokio::time::timeout(Duration::from_secs(2), send.send(Err(error))).await;
        }
    });
    let first = match tokio::time::timeout(Duration::from_secs(15), receive.recv()).await {
        Ok(Some(Ok(bytes))) => bytes,
        _ => {
            return Err(Error::Unavailable(
                "Live audio could not start. Check the original audio and the server encoder."
                    .into(),
            )
            .into());
        }
    };
    let body =
        futures_util::stream::unfold((Some(first), receive), |(first, mut receive)| async move {
            match first {
                Some(bytes) => Some((Ok(bytes), (None, receive))),
                None => receive.recv().await.map(|result| (result, (None, receive))),
            }
        });
    builder
        .body(Body::from_stream(body))
        .map_err(|e| Error::Storage(e.to_string()).into())
}
