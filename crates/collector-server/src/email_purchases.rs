use super::*;
use collector_app::{
    ConfigureMail, ConnectBridge, MailConnection, MailProfile, MailSetupStatus, PurchaseInbox,
    PurchaseInboxView, ReopenPurchase, RepairBridge, ReviewPurchase,
};
#[utoipa::path(get,path="/api/v1/mail/setup",responses((status=200,body=MailSetupStatus),(status=401),(status=403)))]
pub(super) async fn mail_setup(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<MailSetupStatus>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.mail_setup_status(&actor)).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/mail/setup",request_body=ConnectBridge,responses((status=200,body=MailProfile),(status=401),(status=403),(status=409),(status=503)))]
pub(super) async fn connect_bridge(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<ConnectBridge>,
) -> ApiResult<Json<MailProfile>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || state.service.connect_mail_bridge(&actor, command, &context)).await?,
    ))
}
#[derive(Deserialize)]
pub(super) struct InboxQuery {
    #[serde(default)]
    after: String,
    #[serde(default)]
    view: PurchaseInboxView,
}
#[utoipa::path(get,path="/api/v1/mail/profiles",responses((status=200,body=Vec<MailProfile>),(status=401)))]
pub(super) async fn mail_profiles(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<MailProfile>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.mail_profiles(&actor)).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/mail/connections",responses((status=200,body=Vec<MailConnection>),(status=401)))]
pub(super) async fn mail_connections(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<MailConnection>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.mail_connections(&actor)).await?,
    ))
}
#[utoipa::path(put,path="/api/v1/mail/connections/{id}",params(("id"=String,Path)),request_body=ConfigureMail,responses((status=200,body=MailConnection),(status=400),(status=403),(status=409)))]
pub(super) async fn configure_mail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(command): Json<ConfigureMail>,
) -> ApiResult<Json<MailConnection>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || state.service.configure_mail(&actor, &id, command, &context)).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/collections/{id}/purchase-inbox",params(("id"=CollectionId,Path),("after"=Option<String>,Query),("view"=Option<PurchaseInboxView>,Query)),responses((status=200,body=PurchaseInbox),(status=403)))]
pub(super) async fn purchase_inbox(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CollectionId>,
    Query(query): Query<InboxQuery>,
) -> ApiResult<Json<PurchaseInbox>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .purchase_inbox(&actor, id, &query.after, query.view)
        })
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/purchase-suggestions/{id}/accept",params(("id"=String,Path)),request_body=ReviewPurchase,responses((status=200,body=Item),(status=403),(status=409)))]
pub(super) async fn review_purchase(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(command): Json<ReviewPurchase>,
) -> ApiResult<Json<Item>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .review_purchase(&actor, &id, command, &context)
        })
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/purchase-suggestions/{id}/dismiss",params(("id"=String,Path)),request_body=Revision,responses((status=200),(status=403),(status=409)))]
pub(super) async fn dismiss_purchase(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(expected): Json<collector_core::Revision>,
) -> ApiResult<Json<()>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    blocking(move || {
        state
            .service
            .dismiss_purchase(&actor, &id, expected, &context)
    })
    .await?;
    Ok(Json(()))
}
pub(super) async fn purchase_inbox_js() -> impl IntoResponse {
    (
        [("content-type", "text/javascript; charset=utf-8")],
        include_str!("../../../web/dist/purchase-inbox.js"),
    )
}

#[utoipa::path(post,path="/api/v1/mail/profiles/{id}/repair",params(("id"=String,Path)),request_body=RepairBridge,responses((status=200,body=MailProfile),(status=400),(status=403),(status=409),(status=503)))]
pub(super) async fn repair_bridge(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(command): Json<RepairBridge>,
) -> ApiResult<Json<MailProfile>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .repair_mail_bridge(&actor, &id, command, &context)
        })
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/purchase-suggestions/{id}/reopen",params(("id"=String,Path)),request_body=ReopenPurchase,responses((status=200,body=Item),(status=403),(status=409)))]
pub(super) async fn reopen_purchase(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(command): Json<ReopenPurchase>,
) -> ApiResult<Json<Item>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .reopen_purchase(&actor, &id, command, &context)
        })
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/purchase-suggestions/{id}/restore",params(("id"=String,Path)),request_body=Revision,responses((status=200),(status=403),(status=409)))]
pub(super) async fn restore_purchase(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(expected): Json<collector_core::Revision>,
) -> ApiResult<Json<()>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    blocking(move || {
        state
            .service
            .restore_purchase(&actor, &id, expected, &context)
    })
    .await?;
    Ok(Json(()))
}
