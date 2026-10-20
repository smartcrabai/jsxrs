mod discovery;
mod handler;
mod layout;

pub use discovery::RouteEntry;

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::config::{CheckedRenderContext, RenderConfig};
use crate::error::JsxrsError;

/// A file-system based router for Axum, inspired by Next.js App Router.
///
/// # Example
///
/// ```no_run
/// use jsxrs::router::JsxRouter;
/// use jsxrs::RenderConfig;
///
/// # async fn example() {
/// let app = JsxRouter::new("./app")
///     .with_config(RenderConfig { tailwind: true, ..Default::default() })
///     .into_router()
///     .expect("failed to build router");
///
/// let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
/// axum::serve(listener, app).await.unwrap();
/// # }
/// ```
pub struct JsxRouter {
    app_dir: PathBuf,
    config: RenderConfig,
    route_entries: Option<Vec<RouteEntry>>,
    checked_context: CheckedRenderContext,
}

impl JsxRouter {
    /// Create a router builder that discovers routes from the filesystem at runtime.
    pub fn new(app_dir: impl Into<PathBuf>) -> Self {
        Self {
            app_dir: app_dir.into(),
            config: RenderConfig::default(),
            route_entries: None,
            checked_context: CheckedRenderContext::default(),
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __from_checked_entries(route_entries: Vec<RouteEntry>) -> Self {
        Self {
            app_dir: PathBuf::new(),
            config: RenderConfig::default(),
            route_entries: Some(route_entries),
            checked_context: CheckedRenderContext::default(),
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __from_checked_sources(
        route_entries: Vec<RouteEntry>,
        checked_sources: Arc<crate::__private::CheckedSourceSet>,
        source_root: PathBuf,
    ) -> Self {
        Self {
            app_dir: PathBuf::new(),
            config: RenderConfig::default(),
            route_entries: Some(route_entries),
            checked_context: CheckedRenderContext::default()
                .__with_checked_sources(source_root, checked_sources),
        }
    }

    #[must_use]
    pub fn with_config(mut self, config: RenderConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the runtime root used to validate checked route sources.
    #[must_use]
    pub fn with_source_root(mut self, source_root: impl Into<PathBuf>) -> Self {
        self.checked_context = self.checked_context.with_source_root(source_root);
        self
    }

    /// Build an Axum `Router` from the precomputed `routes!` entries or runtime discovery.
    ///
    /// Routers created by `routes!` use their precomputed entries. Routers created by
    /// [`JsxRouter::new`] discover routes from the filesystem at runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if runtime route discovery fails to scan the app directory.
    pub fn into_router(self) -> Result<Router, JsxrsError> {
        let routes = match self.route_entries {
            Some(entries) => entries,
            None => discovery::scan_routes(&self.app_dir).map_err(JsxrsError::Io)?,
        };

        let config = Arc::new(self.config);
        let checked_context = Arc::new(self.checked_context);
        let mut axum_router = Router::new();

        for entry in routes {
            let route_path = entry.axum_path.clone();
            let is_dynamic = route_path.contains('{');
            let route_handler = handler::RouteHandler {
                entry: Arc::new(entry),
                config: Arc::clone(&config),
                checked_context: Arc::clone(&checked_context),
            };

            if is_dynamic {
                axum_router = axum_router.route(
                    &route_path,
                    get(
                        move |path: axum::extract::Path<
                            std::collections::HashMap<String, String>,
                        >| async move {
                            handler::handle_dynamic(path, &route_handler)
                        },
                    ),
                );
            } else {
                axum_router = axum_router.route(
                    &route_path,
                    get(move || async move { handler::handle_static(&route_handler) }),
                );
            }
        }

        Ok(axum_router)
    }
}
