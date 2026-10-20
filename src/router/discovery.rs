pub use jsxrs_core::discovery::{RouteEntry, scan_routes};

#[cfg(test)]
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_app(base: &Path) -> Result<(), std::io::Error> {
        let dirs = ["", "about", "blog", "blog/[slug]", "(marketing)/contact"];
        for d in &dirs {
            let dir = base.join(d);
            fs::create_dir_all(&dir)?;
            fs::write(
                dir.join("page.tsx"),
                "export default function Page() { return <div>test</div>; }",
            )?;
        }
        // Root layout
        fs::write(
            base.join("layout.tsx"),
            "export default function Layout() { return <html><body>{props.children}</body></html>; }",
        )?;
        // Blog layout
        fs::write(
            base.join("blog/layout.tsx"),
            "export default function BlogLayout() { return <div class=\"blog\">{props.children}</div>; }",
        )?;
        Ok(())
    }

    #[test]
    fn test_scan_routes() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = std::env::temp_dir().join("jsxrs_test_discovery");
        let _ = fs::remove_dir_all(&tmp);
        create_test_app(&tmp)?;

        let routes = scan_routes(&tmp)?;

        let paths: Vec<&str> = routes.iter().map(|r| r.axum_path.as_str()).collect();
        assert!(paths.contains(&"/"), "should have root route");
        assert!(paths.contains(&"/about"), "should have /about");
        assert!(paths.contains(&"/blog"), "should have /blog");
        assert!(
            paths.contains(&"/blog/{slug}"),
            "should have /blog/{{slug}}"
        );
        assert!(
            paths.contains(&"/contact"),
            "should have /contact (group stripped)"
        );

        // Check layouts
        let blog_slug = routes
            .iter()
            .find(|r| r.axum_path == "/blog/{slug}")
            .ok_or("blog/{slug} route not found")?;
        assert_eq!(
            blog_slug.layouts.len(),
            2,
            "should have root + blog layouts"
        );

        let contact = routes
            .iter()
            .find(|r| r.axum_path == "/contact")
            .ok_or("contact route not found")?;
        assert_eq!(contact.layouts.len(), 1, "should have root layout only");

        let _ = fs::remove_dir_all(&tmp);
        Ok(())
    }
}
