//! The echo strategy's own endpoints. Ours, not upstream's: kept out of `naming.rs` so a
//! merge from upstream does not run into them.

use axum::Json;
use axum::extract::{Path, Query, State};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::ApiResult;
use super::extract::{Credentials, ShareQuery, read_map_as, require_actor};
use crate::app::AppState;
use crate::maps::naming::echo::{EchoRowAlias, NameGhost, NamedGhost};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(rows))
        .routes(routes!(name_ghost))
}

/// `GET /api/maps/{id}/naming/echo/rows/{system}`: the name of every wormhole row scanned
/// in a system, for the signature list. Empty on a map that does not use echo.
#[utoipa::path(
    get,
    path = "/api/maps/{id}/naming/echo/rows/{system}",
    tag = "naming",
    params(
        ("id" = i64, Path, description = "The map"),
        ("system" = i64, Path, description = "The placement the rows were scanned in"),
        ShareQuery,
    ),
    responses((status = 200, body = Vec<EchoRowAlias>, description = "OK"), super::SharedReadErrors),
    security((), ("bearer" = []), ("session" = [])),
)]
pub async fn rows(
    State(state): State<AppState>,
    creds: Credentials,
    Path((map_id, system)): Path<(i64, i64)>,
    Query(share): Query<ShareQuery>,
) -> ApiResult<Vec<EchoRowAlias>> {
    let reader = read_map_as(&state, &creds, map_id, &share).await?;
    Ok(Json(
        crate::maps::naming::echo::rows(&state.db, reader, map_id, system).await?,
    ))
}

/// `POST /api/maps/{id}/naming/echo/ghost`: name the ghost a scan leads to, now that its
/// type says where, and write the name.
#[utoipa::path(
    post,
    path = "/api/maps/{id}/naming/echo/ghost",
    tag = "naming",
    params(("id" = i64, Path, description = "The map"), ShareQuery),
    request_body = NameGhost,
    responses((status = 200, body = NamedGhost, description = "OK"), super::SharedReadErrors),
)]
pub async fn name_ghost(
    State(state): State<AppState>,
    creds: Credentials,
    Path(map_id): Path<i64>,
    Query(share): Query<ShareQuery>,
    Json(cmd): Json<NameGhost>,
) -> ApiResult<NamedGhost> {
    let actor = require_actor(&state.db, &creds).await?;
    let reader = read_map_as(&state, &creds, map_id, &share).await?;
    Ok(Json(
        crate::maps::naming::echo::name_ghost(&state.db, reader, actor, map_id, cmd).await?,
    ))
}
