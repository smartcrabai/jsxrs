use std::borrow::Cow;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A discovered route entry.
#[derive(Debug, Clone)]
pub struct RouteEntry {
    /// Axum path pattern (for example, `/blog/{slug}`).
    pub axum_path: String,
    /// Absolute path to the page source.
    pub page_file: PathBuf,
    /// Layout files from root to innermost.
    pub layouts: Vec<PathBuf>,
}

/// Scan a directory tree for Next.js-style routes.
///
/// # Errors
///
/// Returns an error for directory I/O failures, invalid route names, or route collisions.
pub fn scan_routes(app_dir: &Path) -> Result<Vec<RouteEntry>, io::Error> {
    let app_dir = app_dir.canonicalize()?;
    if !app_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            format!("app route root is not a directory: {}", app_dir.display()),
        ));
    }
    let mut entries = Vec::new();
    collect_routes(&app_dir, "", &[], &mut entries)?;
    entries.sort_by_cached_key(|entry| {
        (
            entry.axum_path.matches('{').count(),
            entry.axum_path.clone(),
        )
    });
    validate_routes(&entries)?;
    Ok(entries)
}

fn collect_routes(
    current_dir: &Path,
    url_prefix: &str,
    parent_layouts: &[PathBuf],
    entries: &mut Vec<RouteEntry>,
) -> Result<(), io::Error> {
    let layouts = match find_file(current_dir, "layout")? {
        Some(layout) => {
            let mut layouts = parent_layouts.to_vec();
            layouts.push(layout);
            Cow::Owned(layouts)
        }
        None => Cow::Borrowed(parent_layouts),
    };
    if let Some(page) = find_file(current_dir, "page")? {
        entries.push(RouteEntry {
            axum_path: if url_prefix.is_empty() {
                "/".to_owned()
            } else {
                url_prefix.to_owned()
            },
            page_file: page,
            layouts: layouts.to_vec(),
        });
    }

    let mut subdirectories = Vec::new();
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            subdirectories.push(entry);
        }
    }
    subdirectories.sort_by_key(fs::DirEntry::file_name);
    for entry in subdirectories {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        let segment = dir_name_to_segment(&name)?;
        let child_prefix = match segment {
            Segment::Static(value) => format!("{url_prefix}/{value}"),
            Segment::Dynamic(parameter) => format!("{url_prefix}/{{{parameter}}}"),
            Segment::CatchAll(parameter) => format!("{url_prefix}/{{*{parameter}}}"),
            Segment::Group => url_prefix.to_owned(),
        };
        collect_routes(&entry.path(), &child_prefix, &layouts, entries)?;
    }
    Ok(())
}

#[derive(Debug)]
enum Segment<'a> {
    Static(&'a str),
    Dynamic(&'a str),
    CatchAll(&'a str),
    Group,
}

fn dir_name_to_segment(name: &str) -> Result<Segment<'_>, io::Error> {
    if name.starts_with('(') || name.ends_with(')') {
        if name.starts_with('(') && name.ends_with(')') && name.len() > 2 {
            return Ok(Segment::Group);
        }
        return Err(invalid_route(format!(
            "invalid route group directory: {name}"
        )));
    }
    if name.starts_with("[...") && name.ends_with(']') {
        let parameter = &name[4..name.len() - 1];
        if valid_parameter(parameter) {
            return Ok(Segment::CatchAll(parameter));
        }
        return Err(invalid_route(format!(
            "invalid catch-all route parameter: {name}"
        )));
    }
    if name.starts_with('[') || name.ends_with(']') {
        if name.starts_with('[') && name.ends_with(']') {
            let parameter = &name[1..name.len() - 1];
            if valid_parameter(parameter) {
                return Ok(Segment::Dynamic(parameter));
            }
            return Err(invalid_route(format!(
                "invalid dynamic route parameter: {name}"
            )));
        }
        return Err(invalid_route(format!(
            "invalid dynamic route directory: {name}"
        )));
    }
    Ok(Segment::Static(name))
}

fn valid_parameter(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn find_file(directory: &Path, base_name: &str) -> Result<Option<PathBuf>, io::Error> {
    for extension in ["tsx", "jsx"] {
        let path = directory.join(format!("{base_name}.{extension}"));
        match fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => return Ok(Some(path)),
            Ok(_) => {
                return Err(invalid_route(format!(
                    "route source is not a file: {}",
                    path.display()
                )));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(None)
}

fn validate_routes(entries: &[RouteEntry]) -> Result<(), io::Error> {
    let mut patterns = HashSet::<&str>::new();
    let mut structural_patterns = HashSet::new();
    for entry in entries {
        if !patterns.insert(entry.axum_path.as_str()) {
            return Err(invalid_route(format!(
                "duplicate route: {}",
                entry.axum_path
            )));
        }
        let mut params = HashSet::new();
        let mut segments = entry
            .axum_path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .peekable();
        let mut catch_all_seen = false;
        let mut structural = String::new();
        while let Some(segment) = segments.next() {
            if segment.starts_with("{*") && segment.ends_with('}') {
                let parameter = &segment[2..segment.len() - 1];
                if !valid_parameter(parameter)
                    || !params.insert(parameter)
                    || segments.peek().is_some()
                {
                    return Err(invalid_route(format!(
                        "invalid catch-all route: {}",
                        entry.axum_path
                    )));
                }
                catch_all_seen = true;
                if !structural.is_empty() {
                    structural.push('/');
                }
                structural.push_str("{*}");
            } else if segment.starts_with('{') && segment.ends_with('}') {
                let parameter = &segment[1..segment.len() - 1];
                if !valid_parameter(parameter) || !params.insert(parameter) || catch_all_seen {
                    return Err(invalid_route(format!(
                        "invalid dynamic route: {}",
                        entry.axum_path
                    )));
                }
                if !structural.is_empty() {
                    structural.push('/');
                }
                structural.push_str("{}");
            } else {
                if !structural.is_empty() {
                    structural.push('/');
                }
                structural.push_str(segment);
            }
        }
        if !structural_patterns.insert(structural) {
            return Err(invalid_route(format!(
                "routes have the same dynamic shape: {}",
                entry.axum_path
            )));
        }
    }
    Ok(())
}

fn invalid_route(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
