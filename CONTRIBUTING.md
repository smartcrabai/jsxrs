# Contributing

Thank you for your interest in contributing.

## How to Contribute

1. Open an issue or discussion for non-trivial changes before starting work.
2. Fork the repository and create a topic branch.
3. Keep changes focused and include tests or documentation updates when appropriate.
4. Run the repository's formatter, linter, and test commands before opening a pull request.
5. Open a pull request with a clear summary and any relevant context.

## Pull Request Guidelines

- Keep pull requests small and reviewable.
- Explain why the change is needed, not only what changed.
- Link related issues when applicable.
- Be respectful and constructive in reviews and discussions.

## Publishing crates (maintainers)

crates.io Trusted Publishing (OIDC) cannot create crate names that are not yet registered. For the initial publication of `jsxrs-core`, `jsxrs-macros`, and `jsxrs`:

1. Create a short-lived crates.io API token that can create and publish new crates; a token limited to existing crates will not work. Add it as the repository Actions secret `CARGO_REGISTRY_TOKEN`.
2. Ensure the triggering `v<version>` tag points to a commit containing the updated `.github/workflows/publish-crates.yml`. The tag version must match every publishable workspace package version.
3. After the crate names are registered, configure a Trusted Publisher on each crate for GitHub owner `smartcrabai`, repository `jsxrs`, and workflow filename `publish-crates.yml`. Remove the `CARGO_REGISTRY_TOKEN` secret and revoke the temporary token so future releases use OIDC.

See the [crates.io Trusted Publishing documentation](https://crates.io/docs/trusted-publishing). Re-running an old failed run does not load updated workflow YAML. If needed, from the tagged checkout run `cargo login` (or set `CARGO_REGISTRY_TOKEN`), then `cargo publish --package jsxrs-core --package jsxrs-macros --package jsxrs` to bootstrap crate creation before rerunning the tag's workflow.

## Security Issues

Please do not disclose security vulnerabilities in public issues. See `SECURITY.md` for reporting instructions.
