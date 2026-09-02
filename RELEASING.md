# Release the agent-browser plugin

The npm package name is `agent-browser-plugin-browser-jr`. Keep its version equal to the Cargo package version.

## Before publishing

1. Add the repository license selected by the owner.
2. Decide which native platforms the release supports.
3. Build and test one `browser-jr` binary for every supported platform.
4. Choose a binary distribution method. The source package currently resolves `BROWSER_JR_BIN`, a packaged release binary, or `browser-jr` on `PATH`.
5. Verify the intended npm package name is still available.

Do not publish only the JavaScript adapter while claiming that it contains a portable native browser.jr binary.

## Validate the source package

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
npm run check:versions
npm test
npm run pack:check

cd benchmarks
pnpm install --frozen-lockfile
pnpm test
pnpm bench
```

Test the packed plugin from a temporary directory:

```sh
cargo build --release --bin browser-jr
npm pack --pack-destination /tmp

work=$(mktemp -d)
cd "$work"
export BROWSER_JR_BIN=/absolute/path/to/browser.jr/target/release/browser-jr
agent-browser plugin add   file:/tmp/agent-browser-plugin-browser-jr-0.1.0.tgz
agent-browser plugin show browser-jr
agent-browser plugin run browser-jr browserjr.session   --payload '{"commands":["open https://example.com","snapshot -i","get title"]}'
```

Manifest discovery must complete without compiling Rust or producing extra stdout.

## Publish

After the license and binary policy are complete:

1. Update `Cargo.toml` and `package.json` to the same version.
2. Update the README evidence date and benchmark result.
3. Commit from a clean worktree.
4. Tag the commit as `v<version>`.
5. Publish the tested npm artifact with provenance.
6. Create a GitHub release with native binaries and SHA-256 checksums when binaries are distributed separately.
7. Repeat the temporary-directory plugin smoke test against the published version.

Recommend a pinned plugin version in automation:

```sh
agent-browser plugin add agent-browser-plugin-browser-jr@0.1.0
```

## Roll back

If the npm package is broken, deprecate the affected version with an exact reason. Publish a corrected patch version. Do not replace an existing tarball or move an existing Git tag.
