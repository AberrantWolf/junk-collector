//! Typed remote library operations sharing the server's transport contracts.
use crate::{EnrolledClient, client_error, invalid_url};
use collector_api_types::*;
use collector_core::*;
use serde::de::DeserializeOwned;

impl EnrolledClient {
    pub async fn collections(&self) -> Result<Vec<ClientCollection>> {
        self.read_json("api/v1/client/collections", &[]).await
    }
    pub async fn albums(&self, query: &BrowseQuery) -> Result<ClientAlbumPage> {
        decode_request(
            self.get(self.api_url("api/v1/client/albums")?)?
                .query(query),
        )
        .await
    }
    pub async fn songs(
        &self,
        collection: CollectionId,
        query: &MusicSearchQuery,
    ) -> Result<Page<MusicSearchHit>> {
        decode_request(
            self.get(self.api_url(&format!("api/v1/collections/{collection}/music-search"))?)?
                .query(query),
        )
        .await
    }
    pub async fn principal(&self) -> Result<Principal> {
        self.read_json("api/v1/me", &[]).await
    }

    pub fn api_url(&self, path: &str) -> Result<url::Url> {
        self.enrolled
            .pairing
            .endpoints
            .iter()
            .find(|v| v.healthy && v.url.scheme() == "https")
            .ok_or_else(|| Error::Unavailable("no healthy enrolled endpoint".into()))?
            .url
            .join(path)
            .map_err(invalid_url)
    }

    async fn read_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        decode_request(self.get(self.api_url(path)?)?.query(query)).await
    }

    pub async fn lists(
        &self,
        collection: CollectionId,
        query: &CollectionListsQuery,
    ) -> Result<Page<CollectionListSummary>> {
        decode_request(
            self.get(self.api_url(&format!("api/v1/collections/{collection}/lists"))?)?
                .query(query),
        )
        .await
    }

    pub async fn list(
        &self,
        collection: CollectionId,
        list: CollectionListId,
        query: &ListEntriesQuery,
    ) -> Result<CollectionListView> {
        decode_request(
            self.get(self.api_url(&format!("api/v1/collections/{collection}/lists/{list}"))?)?
                .query(query),
        )
        .await
    }

    /// Freeze a revision-consistent playback snapshot. A conflict is returned;
    /// callers can refresh rather than combining entries from different revisions.
    pub async fn list_snapshot(
        &self,
        collection: CollectionId,
        list: CollectionListId,
    ) -> Result<CollectionListView> {
        let mut view = self
            .list(
                collection,
                list,
                &ListEntriesQuery {
                    cursor: None,
                    limit: Some(100),
                },
            )
            .await?;
        let mut cursors = std::collections::BTreeSet::new();
        let mut entries = view
            .entries
            .items
            .iter()
            .map(|v| v.id)
            .collect::<std::collections::BTreeSet<_>>();
        while let Some(cursor) = view.entries.next_cursor.take() {
            if !cursors.insert(cursor.clone()) || cursors.len() > 100 {
                return Err(Error::Unavailable("Invalid playlist pagination".into()));
            }
            let page = self
                .list(
                    collection,
                    list,
                    &ListEntriesQuery {
                        cursor: Some(cursor),
                        limit: Some(100),
                    },
                )
                .await?;
            if page.list.id != list
                || page.list.collection_id != collection
                || page.list.revision != view.list.revision
            {
                return Err(Error::Conflict);
            }
            if view.entries.items.len() + page.entries.items.len() > 5000 {
                return Err(Error::Invalid("playlist exceeds snapshot bound".into()));
            }
            for entry in &page.entries.items {
                if !entries.insert(entry.id) {
                    return Err(Error::Unavailable(
                        "Repeated playlist occurrence in pagination".into(),
                    ));
                }
            }
            view.entries.items.extend(page.entries.items);
            view.entries.next_cursor = page.entries.next_cursor;
        }
        if view.list.id != list
            || view.list.collection_id != collection
            || view.entries.items.len() != view.list.entry_count as usize
        {
            return Err(Error::Unavailable("Incomplete playlist snapshot".into()));
        }
        Ok(view)
    }

    pub async fn create_list(
        &self,
        collection: CollectionId,
        input: &CreateCollectionList,
        key: &str,
    ) -> Result<CollectionListSummary> {
        decode_request(
            self.request(
                reqwest::Method::POST,
                self.api_url(&format!("api/v1/collections/{collection}/lists"))?,
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }

    pub async fn update_list(
        &self,
        collection: CollectionId,
        list: CollectionListId,
        input: &UpdateCollectionList,
        key: &str,
    ) -> Result<CollectionListSummary> {
        decode_request(
            self.request(
                reqwest::Method::PUT,
                self.api_url(&format!("api/v1/collections/{collection}/lists/{list}"))?,
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }

    pub async fn album_playback(
        &self,
        item: ItemId,
        source: Option<RepresentationId>,
    ) -> Result<AlbumPlaybackSnapshot> {
        let query = source
            .map(|v| vec![("source", v.to_string())])
            .unwrap_or_default();
        self.read_json(&format!("api/v1/items/{item}/client-playback"), &query)
            .await
    }

    pub async fn library_summary(&self, query: &BrowseQuery) -> Result<CatalogSummary> {
        decode_request(
            self.get(self.api_url("api/v1/client/library")?)?
                .query(query),
        )
        .await
    }
    pub async fn track_summary(&self, item: ItemId) -> Result<TrackSummary> {
        self.read_json(&format!("api/v1/items/{item}/client-tracks/summary"), &[])
            .await
    }
    pub async fn track_page(&self, item: ItemId, query: &TrackPageQuery) -> Result<TrackPage> {
        let page = decode_request(
            self.get(self.api_url(&format!("api/v1/items/{item}/client-tracks"))?)?
                .query(query),
        )
        .await?;
        validate_track_page(query, &page)?;
        Ok(page)
    }
    pub async fn music_tracks(&self, item: ItemId) -> Result<Vec<MusicTrackAvailability>> {
        let mut page = self.track_page(item, &TrackPageQuery::default()).await?;
        if page.summary.pending {
            return Err(Error::Unavailable("Album tracks are updating".into()));
        }
        let expected = page.summary.clone();
        let mut items = Vec::new();
        loop {
            validate_track_continuation(
                &expected,
                &page.summary,
                items.len(),
                page.items.len(),
                page.next_cursor,
            )?;
            items.extend(page.items);
            let Some(cursor) = page.next_cursor else {
                break;
            };
            page = self
                .track_page(
                    item,
                    &TrackPageQuery {
                        revision: Some(expected.tracks_revision.clone()),
                        cursor: Some(cursor),
                        max_bytes: None,
                    },
                )
                .await?;
        }
        Ok(items)
    }
}

// Validate raw pages too: native album-open reads consume them directly.
fn validate_track_page(query: &TrackPageQuery, page: &TrackPage) -> Result<()> {
    if page.summary.pending {
        if !page.items.is_empty()
            || page.next_cursor.is_some()
            || page.summary.total_items.is_some()
            || !page.summary.tracks_revision.is_empty()
        {
            return Err(Error::Invalid("invalid pending track page".into()));
        }
        if query.revision.is_some() || query.cursor.is_some() {
            return Err(Error::Conflict);
        }
        return Ok(());
    }
    if query
        .revision
        .as_ref()
        .is_some_and(|revision| revision != &page.summary.tracks_revision)
    {
        return Err(Error::Conflict);
    }
    validate_track_continuation(
        &page.summary,
        &page.summary,
        query.cursor.unwrap_or(0) as usize,
        page.items.len(),
        page.next_cursor,
    )
}

/// A continuation must advance within one complete, bounded album revision.
fn validate_track_continuation(
    expected: &TrackSummary,
    actual: &TrackSummary,
    offset: usize,
    count: usize,
    next: Option<u32>,
) -> Result<()> {
    if actual.pending
        || actual.tracks_revision != expected.tracks_revision
        || actual.total_items != expected.total_items
    {
        return Err(Error::Conflict);
    }
    let total = expected
        .total_items
        .ok_or_else(|| Error::Invalid("missing track total".into()))?;
    if expected.tracks_revision.is_empty() || total > 10000 {
        return Err(Error::Invalid(
            "invalid or oversized album track snapshot".into(),
        ));
    }
    let end = offset
        .checked_add(count)
        .ok_or_else(|| Error::Invalid("album track bound exceeded".into()))?;
    let valid = end as u64 <= total
        && match next {
            Some(cursor) => count > 0 && cursor as usize == end && (end as u64) < total,
            None => end as u64 == total,
        };
    if !valid {
        return Err(Error::Invalid(
            "incomplete or non-advancing track pagination".into(),
        ));
    }
    Ok(())
}

impl EnrolledClient {
    pub async fn thumbnail(&self, collection: CollectionId, asset: AssetId) -> Result<Vec<u8>> {
        self.thumbnail_at(collection, asset, None).await
    }
    pub async fn thumbnail_at(
        &self,
        collection: CollectionId,
        asset: AssetId,
        revision: Option<Revision>,
    ) -> Result<Vec<u8>> {
        let mut response = self
            .get(self.api_url(&format!(
                "api/v1/collections/{collection}/assets/{asset}/thumbnail"
            ))?)?
            .query(
                &revision
                    .map(|revision| vec![("revision", revision)])
                    .unwrap_or_default(),
            )
            .send()
            .await
            .map_err(client_error)?
            .error_for_status()
            .map_err(client_error)?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(client_error)? {
            if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
                return Err(Error::Invalid("artwork exceeds client bound".into()));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}

pub(crate) async fn decode_request<T: DeserializeOwned>(
    request: reqwest::RequestBuilder,
) -> Result<T> {
    let mut response = request.send().await.map_err(client_error)?;
    let status = response.status();
    let maximum = if status.is_success() {
        8 * 1024 * 1024
    } else {
        32 * 1024
    };
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(client_error)? {
        if bytes.len() + chunk.len() > maximum {
            return Err(Error::Unavailable(
                "Collector response exceeds client bound".into(),
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        let message = serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|v| {
                v.get("error")
                    .and_then(|v| v.get("message"))
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "Collector rejected the request".into());
        return Err(match status {
            reqwest::StatusCode::UNAUTHORIZED => Error::Unauthenticated,
            reqwest::StatusCode::FORBIDDEN => Error::Forbidden,
            reqwest::StatusCode::CONFLICT => Error::Conflict,
            reqwest::StatusCode::NOT_FOUND => Error::NotFound,
            reqwest::StatusCode::BAD_REQUEST | reqwest::StatusCode::UNPROCESSABLE_ENTITY => {
                Error::Invalid(message)
            }
            _ => Error::Unavailable("Collector request failed".into()),
        });
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| Error::Unavailable("Collector response was incomplete or invalid".into()))
}

#[cfg(test)]
mod track_pagination_tests {
    use super::*;
    fn summary(total: u64) -> TrackSummary {
        TrackSummary {
            tracks_revision: "album-version".into(),
            total_items: Some(total),
            pending: false,
        }
    }
    #[test]
    fn raw_pages_allow_initial_pending_but_reject_superseded_and_malformed_reads() {
        let mut page = TrackPage {
            summary: TrackSummary {
                tracks_revision: String::new(),
                total_items: None,
                pending: true,
            },
            items: Vec::new(),
            next_cursor: None,
        };
        validate_track_page(&TrackPageQuery::default(), &page).unwrap();
        let query = TrackPageQuery {
            revision: Some("old-version".into()),
            ..Default::default()
        };
        assert!(matches!(
            validate_track_page(&query, &page),
            Err(Error::Conflict)
        ));
        page.next_cursor = Some(0);
        assert!(matches!(
            validate_track_page(&TrackPageQuery::default(), &page),
            Err(Error::Invalid(_))
        ));
        page.next_cursor = None;
        page.summary = summary(0);
        assert!(matches!(
            validate_track_page(&query, &page),
            Err(Error::Conflict)
        ));
        page.summary = summary(3);
        assert!(matches!(
            validate_track_page(&TrackPageQuery::default(), &page),
            Err(Error::Invalid(_))
        ));
    }
    #[test]
    fn accepts_contiguous_complete_pages_and_an_empty_album() {
        let s = summary(3);
        validate_track_continuation(&s, &s, 0, 2, Some(2)).unwrap();
        validate_track_continuation(&s, &s, 2, 1, None).unwrap();
        let empty = summary(0);
        validate_track_continuation(&empty, &empty, 0, 0, None).unwrap();
    }
    #[test]
    fn rejects_loops_skipped_rows_truncation_and_excess_rows() {
        let s = summary(3);
        for (offset, count, next) in [
            (0, 0, Some(0)),
            (2, 0, Some(2)),
            (0, 1, Some(2)),
            (0, 1, None),
            (2, 2, None),
            (0, 3, Some(3)),
        ] {
            assert!(validate_track_continuation(&s, &s, offset, count, next).is_err());
        }
        let oversized = summary(10001);
        assert!(validate_track_continuation(&oversized, &oversized, 0, 1, Some(1)).is_err());
    }
    #[test]
    fn rejects_changed_revision_total_and_pending_projection() {
        let s = summary(3);
        let mut changed = s.clone();
        changed.tracks_revision = "new-version".into();
        assert!(matches!(
            validate_track_continuation(&s, &changed, 0, 1, Some(1)),
            Err(Error::Conflict)
        ));
        changed = summary(4);
        assert!(matches!(
            validate_track_continuation(&s, &changed, 0, 1, Some(1)),
            Err(Error::Conflict)
        ));
        changed = s.clone();
        changed.pending = true;
        assert!(matches!(
            validate_track_continuation(&s, &changed, 0, 0, None),
            Err(Error::Conflict)
        ));
    }
}
