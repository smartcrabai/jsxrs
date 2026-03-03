use encre_css::Config;

/// Generate Tailwind CSS for the given class names.
/// Returns CSS string, or empty string if no classes produce output.
pub fn generate_css(class_names: &[String]) -> String {
    if class_names.is_empty() {
        return String::new();
    }

    let html = class_names
        .iter()
        .map(|c| format!(r#"<div class="{c}"></div>"#))
        .collect::<Vec<_>>()
        .join("\n");

    let config = Config::default();
    let css = encre_css::generate([html.as_str()], &config);

    if css.trim().is_empty() {
        String::new()
    } else {
        css
    }
}
