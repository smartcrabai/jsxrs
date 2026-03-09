mod discovery;
mod handler;
mod layout;

pub use discovery::RouteEntry;

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::config::RenderConfig;
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
}

impl JsxRouter {
    pub fn new(app_dir: impl Into<PathBuf>) -> Self {
        Self {
            app_dir: app_dir.into(),
            config: RenderConfig::default(),
        }
    }

    pub fn with_config(mut self, config: RenderConfig) -> Self {
        self.config = config;
        self
    }

    /// Build an Axum `Router` from the discovered file-system routes.
    pub fn into_router(self) -> Result<Router, JsxrsError> {
        let routes = discovery::scan_routes(&self.app_dir).map_err(JsxrsError::Io)?;

        let config = Arc::new(self.config);
        let mut router = Router::new();

        for entry in routes {
            let route_handler = handler::RouteHandler {
                entry: Arc::new(entry.clone()),
                config: Arc::clone(&config),
            };

            if entry.axum_path.contains('{') {
                router = router.route(
                    &entry.axum_path,
                    get(
                        move |path: axum::extract::Path<
                            std::collections::HashMap<String, String>,
                        >| { handler::handle_dynamic(path, route_handler) },
                    ),
                );
            } else {
                router = router.route(
                    &entry.axum_path,
                    get(move || handler::handle_static(route_handler)),
                );
            }
        }

        Ok(router)
    }
}
