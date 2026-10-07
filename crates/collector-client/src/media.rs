//! Seekable, bounded authenticated media access. Construct and use on a worker.
use crate::EnrolledClient;
use collector_core::{Error, ItemId, RepresentationId, Result};
use reqwest::{StatusCode, header};
use std::{
    io::{self, Read, Seek, SeekFrom},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const CHUNK: usize = 256 * 1024;
#[derive(Clone, Default)]
pub struct MediaCancellation(Arc<AtomicBool>);
impl MediaCancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    fn check(&self) -> io::Result<()> {
        if self.0.load(Ordering::Acquire) {
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "media cancelled",
            ))
        } else {
            Ok(())
        }
    }
}

pub struct RangeReader {
    client: EnrolledClient,
    runtime: tokio::runtime::Runtime,
    url: url::Url,
    length: u64,
    etag: String,
    position: u64,
    buffer_start: u64,
    buffer: Vec<u8>,
    cancellation: MediaCancellation,
}
impl EnrolledClient {
    /// Blocking adapter for decoder workers; never call from an async runtime or UI thread.
    pub fn media_reader(
        &self,
        item: ItemId,
        representation: RepresentationId,
        file_index: usize,
        cancellation: MediaCancellation,
    ) -> Result<RangeReader> {
        // Each worker runtime owns its connection tasks. Sharing a pool with
        // another stopped/current-thread runtime can strand reused connections.
        // Trust and request policy still use the central client implementation.
        let worker_client = EnrolledClient {
            client: self.enrolled.pairing.lan_client()?,
            enrolled: self.enrolled.clone(),
        };
        let url = self.api_url(&format!(
            "api/v1/items/{item}/representations/{representation}/{file_index}"
        ))?;
        let request = worker_client.request(reqwest::Method::HEAD, url.clone())?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| Error::Unavailable("media runtime unavailable".into()))?;
        let response = runtime
            .block_on(cancellable(&cancellation, async {
                request
                    .send()
                    .await
                    .map_err(|_| io::Error::other("media metadata request failed"))
            }))
            .map_err(|error| {
                if error.kind() == io::ErrorKind::Interrupted {
                    Error::JobCancelled
                } else {
                    Error::Unavailable("media metadata request failed".into())
                }
            })?;
        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(Error::CredentialRevoked);
        }
        if response.status() == StatusCode::FORBIDDEN {
            return Err(Error::Forbidden);
        }
        if response.status() != StatusCode::OK {
            return Err(Error::Unavailable("media metadata unavailable".into()));
        }
        let length = response
            .headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| Error::Unavailable("media length missing".into()))?;
        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|v| v.to_str().ok())
            .filter(|s| s.starts_with('"') && s.ends_with('"'))
            .ok_or_else(|| Error::Unavailable("strong media validator missing".into()))?
            .to_owned();
        Ok(RangeReader {
            client: worker_client,
            runtime,
            url,
            length,
            etag,
            position: 0,
            buffer_start: 0,
            buffer: Vec::new(),
            cancellation,
        })
    }
}
impl RangeReader {
    pub fn length(&self) -> u64 {
        self.length
    }
    fn fetch(&mut self) -> io::Result<()> {
        self.cancellation.check()?;
        let end = self
            .position
            .saturating_add(CHUNK as u64 - 1)
            .min(self.length - 1);
        let request = self
            .client
            .request(reqwest::Method::GET, self.url.clone())
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "media authorization expired",
                )
            })?
            .header(header::RANGE, format!("bytes={}-{}", self.position, end))
            .header(header::IF_RANGE, &self.etag);
        let position = self.position;
        let length = self.length;
        let etag = &self.etag;
        let bytes = self
            .runtime
            .block_on(cancellable(&self.cancellation, async {
                let mut response = request
                    .send()
                    .await
                    .map_err(|_| io::Error::other("media range request failed"))?;
                if matches!(
                    response.status(),
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
                ) {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "media access revoked",
                    ));
                }
                validate_range(
                    response.status(),
                    response.headers(),
                    position,
                    end,
                    length,
                    etag,
                )?;
                let wanted = usize::try_from(end - position + 1)
                    .map_err(|_| io::Error::other("range overflow"))?;
                let mut bytes = Vec::with_capacity(wanted);
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(|_| io::Error::other("media read failed"))?
                {
                    if chunk.len() > wanted - bytes.len() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "oversized media range",
                        ));
                    }
                    bytes.extend_from_slice(&chunk);
                }
                if bytes.len() != wanted {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "truncated media range",
                    ));
                }
                Ok(bytes)
            }))?;
        self.cancellation.check()?;
        self.buffer = bytes;
        self.buffer_start = self.position;
        Ok(())
    }
}
/// Dropping the in-flight HTTP future promptly aborts stalled headers/body reads.
async fn cancellable<T>(
    cancel: &MediaCancellation,
    future: impl std::future::Future<Output = io::Result<T>>,
) -> io::Result<T> {
    let bounded = tokio::time::timeout(Duration::from_secs(10), future);
    tokio::pin!(bounded);
    loop {
        cancel.check()?;
        tokio::select! {result=&mut bounded=>return result.map_err(|_|io::Error::new(io::ErrorKind::TimedOut,"media request timed out"))?,_=tokio::time::sleep(Duration::from_millis(20))=>{}}
    }
}
fn validate_range(
    status: StatusCode,
    headers: &header::HeaderMap,
    start: u64,
    end: u64,
    length: u64,
    etag: &str,
) -> io::Result<()> {
    let expected = format!("bytes {start}-{end}/{length}");
    if status != StatusCode::PARTIAL_CONTENT
        || headers
            .get(header::CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            != Some(expected.as_str())
        || headers.get(header::ETAG).and_then(|v| v.to_str().ok()) != Some(etag)
        || headers
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            != Some(end - start + 1)
        || headers.contains_key(header::CONTENT_ENCODING)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "inconsistent media range response",
        ));
    }
    Ok(())
}
impl Read for RangeReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.cancellation.check()?;
        if out.is_empty() || self.position == self.length {
            return Ok(0);
        }
        if self.position < self.buffer_start
            || self.position >= self.buffer_start + self.buffer.len() as u64
        {
            self.fetch()?;
        }
        let offset = (self.position - self.buffer_start) as usize;
        let n = out.len().min(self.buffer.len() - offset);
        out[..n].copy_from_slice(&self.buffer[offset..offset + n]);
        self.position += n as u64;
        Ok(n)
    }
}
impl Seek for RangeReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.cancellation.check()?;
        let value = match from {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::Current(n) => i128::from(self.position) + i128::from(n),
            SeekFrom::End(n) => i128::from(self.length) + i128::from(n),
        };
        if value < 0 || value > i128::from(self.length) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek outside media",
            ));
        }
        self.position = value as u64;
        Ok(self.position)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn range_validation_rejects_changed_ignored_or_inconsistent_responses() {
        let mut h = header::HeaderMap::new();
        h.insert(header::CONTENT_RANGE, "bytes 20-29/100".parse().unwrap());
        h.insert(header::CONTENT_LENGTH, "10".parse().unwrap());
        h.insert(header::ETAG, "\"frozen\"".parse().unwrap());
        assert!(validate_range(StatusCode::PARTIAL_CONTENT, &h, 20, 29, 100, "\"frozen\"").is_ok());
        for status in [
            StatusCode::OK,
            StatusCode::UNAUTHORIZED,
            StatusCode::RANGE_NOT_SATISFIABLE,
        ] {
            assert!(validate_range(status, &h, 20, 29, 100, "\"frozen\"").is_err());
        }
        assert!(
            validate_range(StatusCode::PARTIAL_CONTENT, &h, 20, 29, 101, "\"frozen\"").is_err()
        );
        assert!(
            validate_range(StatusCode::PARTIAL_CONTENT, &h, 20, 29, 100, "\"changed\"").is_err()
        );
        h.insert(header::CONTENT_LENGTH, "9".parse().unwrap());
        assert!(
            validate_range(StatusCode::PARTIAL_CONTENT, &h, 20, 29, 100, "\"frozen\"").is_err()
        );
    }
    #[test]
    fn cancellation_is_shared() {
        let cancel = MediaCancellation::default();
        let sibling = cancel.clone();
        sibling.cancel();
        assert_eq!(
            cancel.check().unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    #[tokio::test]
    async fn stalled_request_is_cancelled_before_its_timeout() {
        let cancel = MediaCancellation::default();
        let signal = cancel.clone();
        let start = std::time::Instant::now();
        let (result, ()) = tokio::join!(
            cancellable(&cancel, std::future::pending::<io::Result<()>>()),
            async move {
                tokio::time::sleep(Duration::from_millis(5)).await;
                signal.cancel();
            }
        );
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Interrupted);
        assert!(start.elapsed() < Duration::from_millis(200));
    }
}
