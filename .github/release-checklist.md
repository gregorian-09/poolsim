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
13. Run `PYTHONPATH=bindings/python python3 -m unittest discover -s bindings/python/tests`.
14. Run `python3 -m build bindings/python`.
15. Run `python3 -m twine check bindings/python/dist/*`.
16. Run `cd bindings/typescript && npm ci`.
17. Run `cd bindings/typescript && npm test`.
18. Run `cd bindings/typescript && npm pack --dry-run`.

## Packaging

1. Run `cargo package -p poolsim-core --allow-dirty`.
2. Confirm the `poolsim-core` package includes its `README.md` and expected source files.
3. Package `poolsim-cli` only after the target `poolsim-core` version exists on crates.io.
4. Package `poolsim-web` only after the target `poolsim-core` version exists on crates.io.
5. Build the Python package from `bindings/python` and confirm both wheel and sdist pass `twine check`.
6. Build the TypeScript package from `bindings/typescript` and confirm `npm pack --dry-run` only includes the compiled package files.

## GitHub Actions Publish Workflow

1. Confirm the repository secret `CARGO_REGISTRY_TOKEN` is present.
2. Confirm the repository secret `PYPI_API_TOKEN` is present for the Python package publish.
3. Confirm the repository secret `NPM_TOKEN` is present for the TypeScript package publish.
4. Commit the release changes.
5. Create a tag matching the root `VERSION` file, for example `v0.2.1`.
6. Push the tag.
7. Confirm the `Publish` workflow starts automatically for that tag.
8. Use `workflow_dispatch` only when you want a manual dry-run of the publish path or a controlled fallback publish.

## Python-Only Publish Workflow

Use `.github/workflows/publish-python.yml` when the Rust crates are already published and only the Python `poolsim` package needs to be published or backfilled.

1. Confirm `PYPI_API_TOKEN` is present.
2. Run the `Publish Python` workflow with the version matching `VERSION`.
3. Keep `dry_run=true` for package validation only.
4. Set `dry_run=false` only when publishing to PyPI.

## Node-Only Publish Workflow

Use `.github/workflows/publish-node.yml` when the Rust crates are already published and only the TypeScript `poolsim` package needs to be published or backfilled.

1. Confirm `NPM_TOKEN` is present.
2. Run the `Publish Node` workflow with the version matching `VERSION`.
3. Keep `dry_run=true` for package validation only.
4. Set `dry_run=false` only when publishing to npm.

## Publish Order

1. Publish `poolsim-core` first:
   `cargo publish -p poolsim-core`
2. Wait for crates.io index propagation.
3. Publish `poolsim-cli`:
   `cargo publish -p poolsim-cli`
4. Publish `poolsim-web`:
   `cargo publish -p poolsim-web`
5. Publish the Python `poolsim` package to PyPI from `bindings/python/dist`.
6. Publish the TypeScript `poolsim` package to npm from `bindings/typescript`.

## Post-Publish

1. Verify docs.rs builds succeeded.
2. Verify `cargo install poolsim-cli` works from crates.io.
3. Verify `pip install poolsim` works from PyPI in a clean virtual environment.
4. Verify `npm install poolsim` works from npm in a clean temporary project.
5. Verify crate, PyPI, and npm pages show the correct README, license, repository, keywords, and categories.
6. Create or update the GitHub Release notes for the pushed version tag after crates.io, PyPI, and npm publication are confirmed.
