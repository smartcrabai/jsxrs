use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde_json::Value;

use crate::config::RenderConfig;

use super::discovery::RouteEntry;
use super::layout::render_with_layouts;

/// Shared state for a route handler.
#[derive(Clone)]
pub(crate) struct RouteHandler {
    pub entry: Arc<RouteEntry>,
    pub config: Arc<RenderConfig>,
}

pub(crate) async fn handle_static(handler: RouteHandler) -> Response {
    render_route(&handler, &Value::Object(serde_json::Map::new()))
}

pub(crate) async fn handle_dynamic(
    Path(params): Path<HashMap<String, String>>,
    handler: RouteHandler,
) -> Response {
    let props = serde_json::to_value(&params).unwrap_or(Value::Object(serde_json::Map::new()));
    render_route(&handler, &props)
}

fn render_route(handler: &RouteHandler, props: &Value) -> Response {
    match render_with_layouts(
        &handler.entry.page_file,
        &handler.entry.layouts,
        props,
        &handler.config,
    ) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Render error: {e}"),
        )
            .into_response(),
    }
}
