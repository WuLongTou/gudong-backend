use axum::{routing::post, Router};

mod group_operation;
mod group_types;

pub(crate) fn router() -> Router {
    Router::new()
        .route("/create", post(group_operation::create_group))
        .route(
            "/query-by-name",
            post(group_operation::query_groups_by_name),
        )
        .route(
            "/query-by-location",
            post(group_operation::query_groups_by_location),
        )
        .route("/join", post(group_operation::join_group))
}
