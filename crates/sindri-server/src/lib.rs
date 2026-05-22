pub mod routes;

use axum::{
    routing::{get, patch, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

pub use routes::{AppState, SharedScene};
pub use tokio::sync::broadcast;

pub async fn serve(state: AppState) -> anyhow::Result<()> {
    let port: u16 = std::env::var("SINDRI_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7878);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/assets/*path", get(routes::get_asset))
        .route("/health", get(routes::health))
        .route("/errors", get(routes::get_errors))
        .route("/scene", get(routes::get_scene).put(routes::put_scene))
        .route("/scene/path", get(routes::get_scene_path))
        .route("/scene/save", post(routes::save_scene))
        .route("/scene/world", get(routes::get_world_settings).patch(routes::patch_world_settings))
        .route("/scene/open", post(routes::open_scene))
        .route(
            "/scene/entity/:id",
            get(routes::get_entity).delete(routes::delete_entity),
        )
        .route(
            "/scene/entity/:id/prefab_source",
            axum::routing::patch(routes::set_entity_prefab_source),
        )
        .route("/scene/entity/:id/name", patch(routes::rename_entity))
        .route(
            "/scene/entity/:id/transform",
            patch(routes::patch_transform),
        )
        .route(
            "/scene/entity/:id/component",
            post(routes::add_component).delete(routes::remove_component),
        )
        .route(
            "/scene/entity/:id/component/:idx",
            patch(routes::patch_component).delete(routes::remove_component_by_idx),
        )
        .route("/scene/entity", post(routes::create_entity))
        .route("/scene/entity/:id/staged", patch(routes::set_entity_staged))
        .route("/scene/staged/commit", post(routes::commit_staged))
        .route("/scene/staged/revert", post(routes::revert_staged))
        .route("/screenshot", get(routes::get_screenshot))
        .route("/stream", get(routes::stream_handler))
        .route("/control", axum::routing::post(routes::post_control))
        .route("/gizmos", axum::routing::post(routes::post_gizmos))
        .route("/input/keys", axum::routing::post(routes::post_keys))
        .route("/scripts", get(routes::list_scripts))
        .route("/script", get(routes::get_script).put(routes::put_script))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
