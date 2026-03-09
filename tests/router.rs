#[cfg(feature = "axum")]
mod tests {
    use axum_test::TestServer;
    use jsxrs::RenderConfig;
    use jsxrs::router::JsxRouter;
    use std::path::PathBuf;

    fn app_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/app")
    }

    fn build_server() -> TestServer {
        let router = JsxRouter::new(app_dir())
            .with_config(RenderConfig::default())
            .into_router()
            .expect("failed to build router");
        TestServer::new(router)
    }

    #[tokio::test]
    async fn test_home_page() {
        let server = build_server();
        let res = server.get("/").await;
        res.assert_status_ok();
        let body = res.text();
        assert!(
            body.contains("<h1>Home</h1>"),
            "body should contain Home heading: {body}"
        );
        // Should be wrapped in root layout
        assert!(
            body.contains("<html>"),
            "body should contain html tag: {body}"
        );
        assert!(
            body.contains("<body>"),
            "body should contain body tag: {body}"
        );
    }

    #[tokio::test]
    async fn test_about_page() {
        let server = build_server();
        let res = server.get("/about").await;
        res.assert_status_ok();
        let body = res.text();
        assert!(body.contains("<h1>About</h1>"), "body: {body}");
        assert!(body.contains("<html>"), "should have root layout: {body}");
    }

    #[tokio::test]
    async fn test_blog_page_with_nested_layout() {
        let server = build_server();
        let res = server.get("/blog").await;
        res.assert_status_ok();
        let body = res.text();
        assert!(body.contains("<h1>Blog</h1>"), "body: {body}");
        // Should have blog layout wrapper
        assert!(
            body.contains("blog-layout"),
            "should have blog layout class: {body}"
        );
        // Should also have root layout
        assert!(body.contains("<html>"), "should have root layout: {body}");
    }

    #[tokio::test]
    async fn test_dynamic_route() {
        let server = build_server();
        let res = server.get("/blog/hello-world").await;
        res.assert_status_ok();
        let body = res.text();
        assert!(
            body.contains("hello-world"),
            "should contain slug param: {body}"
        );
        assert!(
            body.contains("blog-layout"),
            "should have blog layout: {body}"
        );
        assert!(body.contains("<html>"), "should have root layout: {body}");
    }

    #[tokio::test]
    async fn test_route_group() {
        let server = build_server();
        let res = server.get("/contact").await;
        res.assert_status_ok();
        let body = res.text();
        assert!(body.contains("<h1>Contact</h1>"), "body: {body}");
        assert!(body.contains("<html>"), "should have root layout: {body}");
    }

    #[tokio::test]
    async fn test_404() {
        let server = build_server();
        let res = server.get("/nonexistent").await;
        res.assert_status_not_found();
    }
}
