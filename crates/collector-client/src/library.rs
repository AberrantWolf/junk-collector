//! Typed remote library operations sharing the server's transport contracts.
use crate::{EnrolledClient, client_error, invalid_url};
use collector_api_types::*;
use collector_core::*;
use serde::de::DeserializeOwned;

impl EnrolledClient {
    pub async fn collections(&self) -> Result<Vec<ClientCollection>> {
        self.read_json("api/v1/client/collections", &[]).await
    }
    pub async fn albums(&self, query: &BrowseQuery) -> Result<Page<ClientAlbum>> {
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

    pub async fn music_tracks(&self, item: ItemId) -> Result<Vec<MusicTrackAvailability>> {
        self.read_json(&format!("api/v1/items/{item}/client-tracks"), &[])
            .await
    }
}

impl EnrolledClient {
    pub async fn thumbnail(&self, collection: CollectionId, asset: AssetId) -> Result<Vec<u8>> {
        let mut response = self
            .get(self.api_url(&format!(
                "api/v1/collections/{collection}/assets/{asset}/thumbnail"
            ))?)?
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
