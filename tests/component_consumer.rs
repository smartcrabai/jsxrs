#![cfg(feature = "axum")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn checked_component_and_routes_survive_relocation_and_fail_closed_on_drift() -> ConsumerTestResult
{
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::temp_dir().join(format!(
        "jsxrs_checked_consumer_{}_{}",
        std::process::id(),
        nanos
    ));
    let consumer = root.join("consumer");
    let relocated = root.join("relocated-source");
    let binary_dir = root.join("relocated-bin");
    let target = root.join("target");
    let checked_binary = format!("jsxrs_checked_consumer{}", std::env::consts::EXE_SUFFIX);
    fs::create_dir_all(&relocated)?;
    fs::create_dir_all(&binary_dir)?;
    create_checked_consumer(&repository, &consumer)?;

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    build_consumer(
        &cargo,
        &consumer,
        &target,
        "isolated consumer failed to build",
    )?;
    let executable =
        relocate_and_check_drift(&consumer, &relocated, &binary_dir, &target, &checked_binary)?;
    rebuild_after_source_change(
        &cargo,
        &consumer,
        &binary_dir,
        &target,
        &checked_binary,
        &executable,
    )?;
    rebuild_after_new_route(
        &cargo,
        &consumer,
        &binary_dir,
        &target,
        &checked_binary,
        &executable,
    )?;

    fs::remove_dir_all(root)?;
    Ok(())
}

type ConsumerTestResult = Result<(), Box<dyn std::error::Error>>;

fn create_checked_consumer(repository: &Path, consumer: &Path) -> ConsumerTestResult {
    fs::create_dir_all(consumer.join("src"))?;
    fs::create_dir_all(consumer.join("components"))?;
    fs::create_dir_all(consumer.join("app"))?;
    fs::copy(repository.join("Cargo.lock"), consumer.join("Cargo.lock"))?;
    write_checked_consumer_manifest(repository, consumer)?;
    write_checked_consumer_sources(consumer)
}

fn write_checked_consumer_manifest(repository: &Path, consumer: &Path) -> ConsumerTestResult {
    let jsxrs_path = toml_string_path(repository);
    fs::write(
        consumer.join("Cargo.toml"),
        format!(
            "[package]\nname = \"jsxrs_checked_consumer\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\njsxrs = {{ path = \"{jsxrs_path}\", features = [\"axum\"] }}\naxum = \"=0.8.9\"\naxum-test = \"=21.1.0\"\ntokio = {{ version = \"=1.54.1\", features = [\"full\"] }}\n"
        ),
    )?;
    Ok(())
}

fn write_checked_consumer_sources(consumer: &Path) -> ConsumerTestResult {
    fs::write(
        consumer.join("profile.tsx"),
        "import Card from './components/card';\nimport OptionalChild from './components/optional';\nexport interface ProfileProps { name: string; }\nexport default function Profile(props: ProfileProps) { return <div><h1>Hello {props.name}</h1><Card /><OptionalChild /></div>; }\n",
    )?;
    fs::write(
        consumer.join("alias.tsx"),
        "export type AliasProps = { count: number; };\nexport default function Alias(props: AliasProps) { return <span>{props.count}</span>; }\n",
    )?;
    fs::write(
        consumer.join("components/card.jsx"),
        "export default function Card() { return <strong>checked card</strong>; }\n",
    )?;
    fs::write(
        consumer.join("components/optional.tsx"),
        "export interface OptionalChildProps { label?: string; }\nexport default function OptionalChild(props: OptionalChildProps) { return <em>{props.label === null ? 'omitted optional prop' : props.label}</em>; }\n",
    )?;
    fs::write(
        consumer.join("app/page.tsx"),
        "import Card from '../components/card';\nexport default function Page() { return <main><Card /></main>; }\n",
    )?;
    fs::write(
        consumer.join("app/layout.tsx"),
        "export default function Layout(props) { return <html><body>{props.children}</body></html>; }\n",
    )?;
    fs::write(
        consumer.join("src/main.rs"),
        "jsxrs::component!(pub profile, \"profile.tsx\");\n\
         jsxrs::component!(pub alias, \"alias.tsx\");\n\
         #[tokio::main]\n\
         async fn main() -> Result<(), Box<dyn std::error::Error>> {\n\
             let mut args = std::env::args().skip(1);\n\
             let source_root = std::path::PathBuf::from(args.next().ok_or(\"missing source root\")?);\n\
             let mode = args.next().unwrap_or_default();\n\
             let checked_context = jsxrs::CheckedRenderContext::new().with_source_root(source_root.clone());\n\
             let config = jsxrs::RenderConfig { pretty: false, base_dir: None, head_elements: Vec::new(), tailwind: false, fragment: false };\n\
             let alias_props = alias::Props { count: 7.0 };\n\
             let alias_html = alias::render_with_context(&alias_props, &config, &checked_context)?;\n\
             assert!(alias_html.contains(\"<span>7</span>\"), \"{alias_html}\");\n\
             let props = profile::Props { name: String::from(\"Alice\") };\n\
             let component = profile::render_with_context(&props, &config, &checked_context);\n\
             if mode == \"drift\" {\n\
                 assert!(component.is_err(), \"changed checked source must fail closed\");\n\
             } else if mode == \"source_changed\" || mode == \"new_route\" {\n\
                 let html = component?;\n\
                 assert!(html.contains(\"<h1>Hello Alice</h1>\"), \"{html}\");\n\
                 assert!(html.contains(\"rebuilt card\"), \"{html}\");\n\
             } else {\n\
                 let html = component?;\n\
                 assert!(html.contains(\"<h1>Hello Alice</h1>\"), \"{html}\");\n\
                 let expected_card = \"checked card\";\n\
                 assert!(html.contains(expected_card), \"{html}\");\n\
                 assert!(html.contains(\"omitted optional prop\"), \"{html}\");\n\
                 assert!(!html.contains(\"unvalidated candidate\"), \"{html}\");\n\
             }\n\
             let app = jsxrs::routes!(\"app\").with_source_root(source_root).with_config(config).into_router()?;\n\
             let server = axum_test::TestServer::new(app);\n\
             let response = server.get(\"/\").await;\n\
             if mode == \"drift\" {\n\
                 assert_eq!(response.status_code().as_u16(), 500);\n\
             } else {\n\
                 response.assert_status_ok();\n\
                 let html = response.text();\n\
                 let expected_card = if mode == \"source_changed\" || mode == \"new_route\" { \"rebuilt card\" } else { \"checked card\" };\n\
                 assert!(html.contains(expected_card), \"{html}\");\n\
                 assert!(!html.contains(\"unvalidated candidate\"), \"{html}\");\n\
             }\n\
             if mode == \"new_route\" {\n\
                 let new_route = server.get(\"/new\").await;\n\
                 new_route.assert_status_ok();\n\
                 assert!(new_route.text().contains(\"new route\"));\n\
             }\n\
             Ok(())\n\
         }\n",
    )?;
    Ok(())
}

fn build_consumer(
    cargo: &std::ffi::OsStr,
    consumer: &Path,
    target: &Path,
    failure_message: &str,
) -> ConsumerTestResult {
    let build = Command::new(cargo)
        .arg("build")
        .arg("--offline")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(consumer.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", target)
        .current_dir(consumer)
        .output()?;
    assert!(
        build.status.success(),
        "{failure_message}; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    Ok(())
}

fn copy_consumer_binary(target: &Path, binary_dir: &Path, binary_name: &str) -> ConsumerTestResult {
    fs::copy(
        target.join("debug").join(binary_name),
        binary_dir.join(binary_name),
    )?;
    Ok(())
}

fn assert_consumer_runs(
    executable: &Path,
    source_root: &Path,
    binary_dir: &Path,
    mode: Option<&str>,
    failure_message: &str,
) -> ConsumerTestResult {
    let mut command = Command::new(executable);
    command.arg(source_root).current_dir(binary_dir);
    if let Some(mode) = mode {
        command.arg(mode);
    }
    let run = command.output()?;
    assert!(
        run.status.success(),
        "{failure_message}; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    Ok(())
}

fn relocate_and_check_drift(
    consumer: &Path,
    relocated: &Path,
    binary_dir: &Path,
    target: &Path,
    binary_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    for relative in [
        "profile.tsx",
        "alias.tsx",
        "components/card.jsx",
        "components/optional.tsx",
        "app/page.tsx",
        "app/layout.tsx",
    ] {
        let destination = relocated.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(consumer.join(relative), destination)?;
    }
    copy_consumer_binary(target, binary_dir, binary_name)?;

    // This higher-priority candidate was absent during checking. The checked
    // import must remain pinned to card.jsx after relocation.
    fs::write(
        relocated.join("components/card.tsx"),
        "export default function Card() { return <strong>unvalidated candidate</strong>; }\n",
    )?;
    let executable = binary_dir.join(binary_name);
    assert_consumer_runs(
        &executable,
        relocated,
        binary_dir,
        None,
        "relocated consumer failed",
    )?;
    fs::write(
        relocated.join("components/card.jsx"),
        "export default function Card() { return <strong>drifted source</strong>; }\n",
    )?;
    assert_consumer_runs(
        &executable,
        relocated,
        binary_dir,
        Some("drift"),
        "consumer did not reject source drift as expected",
    )?;
    Ok(executable)
}

fn rebuild_after_source_change(
    cargo: &std::ffi::OsStr,
    consumer: &Path,
    binary_dir: &Path,
    target: &Path,
    binary_name: &str,
    executable: &Path,
) -> ConsumerTestResult {
    fs::write(
        consumer.join("components/card.jsx"),
        "export default function Card() { return <strong>rebuilt card</strong>; }\n",
    )?;
    build_consumer(
        cargo,
        consumer,
        target,
        "isolated consumer failed to rebuild after source change",
    )?;
    copy_consumer_binary(target, binary_dir, binary_name)?;
    assert_consumer_runs(
        executable,
        consumer,
        binary_dir,
        Some("source_changed"),
        "rebuilt consumer did not render updated checked sources",
    )
}

fn rebuild_after_new_route(
    cargo: &std::ffi::OsStr,
    consumer: &Path,
    binary_dir: &Path,
    target: &Path,
    binary_name: &str,
    executable: &Path,
) -> ConsumerTestResult {
    fs::create_dir_all(consumer.join("app/new"))?;
    fs::write(
        consumer.join("app/new/page.tsx"),
        "export default function NewPage() { return <main><h1>new route</h1></main>; }\n",
    )?;
    let main_path = consumer.join("src/main.rs");
    let mut main_source = fs::read_to_string(&main_path)?;
    main_source.push_str("\n// Re-expand routes after adding a page fixture.\n");
    fs::write(main_path, main_source)?;
    build_consumer(
        cargo,
        consumer,
        target,
        "isolated consumer failed to rebuild after adding a route",
    )?;
    copy_consumer_binary(target, binary_dir, binary_name)?;
    assert_consumer_runs(
        executable,
        consumer,
        binary_dir,
        Some("new_route"),
        "rebuilt consumer did not render the new route",
    )
}

#[test]
fn checked_component_and_routes_default_to_consumer_root_outside_consumer_cwd()
-> Result<(), Box<dyn std::error::Error>> {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::temp_dir().join(format!(
        "jsxrs_default_root_consumer_{}_{}",
        std::process::id(),
        nanos
    ));
    let consumer = root.join("consumer");
    fs::create_dir_all(consumer.join("src"))?;
    fs::create_dir_all(consumer.join("app"))?;
    fs::copy(repository.join("Cargo.lock"), consumer.join("Cargo.lock"))?;

    fs::write(
        consumer.join("Cargo.toml"),
        format!(
            "[package]\nname = \"jsxrs_default_root_consumer\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\njsxrs = {{ path = \"{}\", features = [\"axum\"] }}\naxum = \"=0.8.9\"\naxum-test = \"=21.1.0\"\ntokio = {{ version = \"=1.54.1\", features = [\"full\"] }}\n",
            toml_string_path(&repository)
        ),
    )?;
    fs::write(
        consumer.join("component.tsx"),
        "export interface Props { name: string; }\nexport default function Component(props: Props) { return <p>Hello {props.name}</p>; }\n",
    )?;
    fs::write(
        consumer.join("app/page.tsx"),
        "export default function Page() { return <main>route from consumer</main>; }\n",
    )?;
    fs::write(
        consumer.join("src/main.rs"),
        "jsxrs::component!(pub standalone, \"component.tsx\");\n\
         #[tokio::main]\n\
         async fn main() -> Result<(), Box<dyn std::error::Error>> {\n\
             let props = standalone::Props { name: String::from(\"outside cwd\") };\n\
             let html = standalone::render(&props, &jsxrs::RenderConfig::default())?;\n\
             assert!(html.contains(\"Hello outside cwd\"), \"{html}\");\n\
             let app = jsxrs::routes!(\"app\").with_config(jsxrs::RenderConfig::default()).into_router()?;\n\
             let server = axum_test::TestServer::new(app);\n\
             let response = server.get(\"/\").await;\n\
             response.assert_status_ok();\n\
             assert!(response.text().contains(\"route from consumer\"));\n\
             Ok(())\n\
         }\n",
    )?;

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let build = Command::new(&cargo)
        .arg("build")
        .arg("--offline")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(consumer.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target"))
        .current_dir(&root)
        .output()?;
    assert!(
        build.status.success(),
        "isolated consumer failed to build outside its directory; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let default_binary = format!(
        "jsxrs_default_root_consumer{}",
        std::env::consts::EXE_SUFFIX
    );
    let run = Command::new(root.join("target/debug").join(default_binary))
        .current_dir(&root)
        .output()?;
    assert!(
        run.status.success(),
        "isolated consumer failed to render outside its directory; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    fs::remove_dir_all(root)?;
    Ok(())
}

fn toml_string_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
