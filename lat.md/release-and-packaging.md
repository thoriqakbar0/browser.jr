# Release and packaging

The npm artifact contains the JavaScript protocol adapter and browser.jr source files. It does not yet contain portable native binaries for supported platforms.

## Version ownership

`Cargo.toml` and `package.json` both carry version `0.1.0`. [`scripts/check-versions.mjs`](../scripts/check-versions.mjs) fails when they differ.

`npm pack` includes the plugin executable, tests, Rust source, Cargo files, README, wrapper script, and the version check. Manifest discovery must not compile Rust or write diagnostics to stdout.

## Runtime dependency

The plugin needs a runnable native `browser-jr`. [[plugin-protocol#Native executable lookup]] defines the lookup order.

A release must either ship platform binaries or document a separate verified installation path. The current package alone is not a portable browser.jr installation.

## Publication gates

[RELEASING.md](../RELEASING.md) requires an owner-selected license, a supported-platform policy, tested native binaries, a distribution method, package-name verification, and provenance.

The repository currently describes these gates but does not enforce them in `package.json`. `publishConfig.access` is `public`, and an npm dry run reaches the public publication step. [BJR-015](../bug-triage.md#bjr-015) owns this conflict.

Until the gates are complete, automation examples must use the local packed tarball rather than imply that version `0.1.0` exists in the npm registry.

## Release evidence

[[verification-map]] separates source-package checks, packed-tarball checks, native runtime checks, and publication safety. Passing tests do not satisfy the license or binary-distribution decisions.
