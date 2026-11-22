# Releasing cargo-c4doc

The crate, CLI, and agent skill are released together from the same source revision.
The GitHub repository is `nwiizo/cargo-c4doc`; the crates.io package is `cargo-c4doc`.
CI validates changes and a publication dry run. It does not publish packages or releases.

## Prepare a release

Use a clean checkout and a Rust toolchain with rustfmt, Clippy, and a C compiler/linker.
Update the package version, the README installation command, and the skill's installation
command together. Keep `Cargo.lock` in the release and install with `--locked`.

Verify the branch and remote, then push the reviewed revision to the public repository.

Run from the source root:

```sh
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all --all-targets
cargo +1.100.0 test --locked --all --all-targets
cargo run --locked -- render examples/c4doc.json --root . --output examples/docs --check
cargo package --list --locked
cargo publish --dry-run --locked
```

Install Rust 1.100.0 with rustup if needed for the minimum-version check. Review the package
list: it must include source, tests, the example model and generated documents (including
`examples/docs/.c4doc-manifest.json`), the entire skill, README, and license. Local build
output and agent configuration are excluded by the manifest's inclusion list. Cargo also
includes its normalized manifest and package metadata. See Cargo's
[packaging documentation](https://doc.rust-lang.org/cargo/commands/cargo-package.html).

The dry run compiles the archive without uploading. Verify installation and the bundled
example from that archive as well; substitute the release version if it is not 0.1.0:

```sh
cargo test --locked --all --all-targets --manifest-path target/package/cargo-c4doc-0.1.0/Cargo.toml
cargo install --path target/package/cargo-c4doc-0.1.0 --locked --root target/install-check
./target/install-check/bin/cargo-c4doc --version
./target/install-check/bin/cargo-c4doc render examples/c4doc.json --root . --output examples/docs --check
```

On Windows, the executable has an `.exe` suffix. The CI package job performs these checks
on Linux; the test matrix additionally covers macOS, Windows, and the minimum Rust version.

## Publish and verify

Confirm that the public repository points to the reviewed revision, the full CI matrix
passes, and the version is available for this project's first upload or belongs to its
maintainers for subsequent releases. Authenticate locally using `cargo login` as described
in the [Cargo publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html).

Publish the same clean revision that passed the dry run:

```sh
cargo publish --locked
```

Once crates.io has indexed the release, verify a fresh installation without the checkout:

```sh
cargo install cargo-c4doc --version 0.1.0 --locked --root target/registry-install-check
./target/registry-install-check/bin/cargo-c4doc --version
```

Tag the published source as `v0.1.0`, push that tag, and create a GitHub Release for it.
The tag's source archive contains the skill and examples; link to
`skills/c4-rust-docs` in the release notes and state that Cargo installs only the CLI.
Describe changes, the Rust requirement, installation, and known limitations.
No prebuilt executables are provided by the current workflow.
