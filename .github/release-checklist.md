# Release Checklist

This checklist is for publishing the current sizing-calculator version of `poolsim`.

## Versioning

1. Update the root `VERSION` file.
2. Run `python3 tools/sync_version.py`.
3. Run `python3 tools/sync_version.py --check` and confirm it passes.
4. Run `cargo check --workspace` to refresh workspace metadata and `Cargo.lock`.
5. Add the new release section to `CHANGELOG.md`.

## Validation

1. Run `cargo test --workspace`.
2. Run `RUSTFLAGS="-D missing_docs" cargo check -p poolsim-core --lib`.
3. Run `RUSTFLAGS="-D missing_docs" cargo check -p poolsim-cli --bin poolsim-cli`.
4. Run `RUSTFLAGS="-D missing_docs" cargo check -p poolsim-web --lib`.
5. Run `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`.
6. Run `cargo test --workspace --doc`.
7. Run `python3 tools/check_docs_folder.py --docs-dir docs`.
8. Run `python3 tools/check_docs_api_coverage.py --docs-dir docs`.
9. Run `cargo test -p poolsim-core --test docs_fixtures`.
10. Run `cargo test -p poolsim-cli --test docs_fixtures`.
11. Run `cargo test -p poolsim-web --test docs_fixtures`.
12. Run `cargo test -p poolsim-web --test http_ws_integration`.

## Packaging

1. Run `cargo package -p poolsim-core --allow-dirty`.
2. Run `cargo package -p poolsim-cli --allow-dirty`.
3. Run `cargo package -p poolsim-web --allow-dirty`.
4. Confirm each package includes its `README.md` and expected source files.

## GitHub Actions Publish Workflow

1. Confirm the repository secret `CARGO_REGISTRY_TOKEN` is present.
2. Commit the release changes.
3. Create a tag matching the root `VERSION` file, for example `v0.1.0`.
4. Push the tag.
5. Confirm the `Publish` workflow starts automatically for that tag.
6. Use `workflow_dispatch` only when you want a manual dry-run or a controlled fallback publish.

## Publish Order

1. Publish `poolsim-core` first:
   `cargo publish -p poolsim-core`
2. Wait for crates.io index propagation.
3. Publish `poolsim-cli`:
   `cargo publish -p poolsim-cli`
4. Publish `poolsim-web`:
   `cargo publish -p poolsim-web`

## Post-Publish

1. Verify docs.rs builds succeeded.
2. Verify `cargo install poolsim-cli` works from crates.io.
3. Verify crate pages show the correct README, license, repository, keywords, and categories.
4. Create or update the GitHub Release notes for the pushed version tag after crates.io publication is confirmed.
