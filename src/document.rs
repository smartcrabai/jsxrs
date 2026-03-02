/// Assemble a full HTML document from head elements and body content.
pub fn build_document(head_html: &str, body_html: &str, pretty: bool) -> String {
    if pretty {
        build_pretty(head_html, body_html)
    } else {
        build_minified(head_html, body_html)
    }
}

fn build_minified(head_html: &str, body_html: &str) -> String {
    format!("<!DOCTYPE html><html><head>{head_html}</head><body>{body_html}</body></html>")
}

fn build_pretty(head_html: &str, body_html: &str) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n");
    out.push_str("<html>\n");

    out.push_str("  <head>\n");
    if !head_html.is_empty() {
        for line in head_html.lines() {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str("  </head>\n");

    out.push_str("  <body>\n");
    out.push_str("    ");
    out.push_str(body_html);
    out.push('\n');
    out.push_str("  </body>\n");

    out.push_str("</html>");
    out
}
