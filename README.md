# aish-plugins

Plugins for [aish](https://github.com/daaquan/aish). Each directory is a source
crate built on `aish plugin install <name>`. Plugins speak the aish stdio ABI
(see `commit/src/protocol.rs`).

## Releases / prebuilt binaries

The aish client looks for plugin binaries on GitHub Releases before falling back
to a local `cargo build`. Release tags and assets must follow this convention:

- Release tag: `{name}-v{version}` (for example, `commit-v0.1.0`)
- Binary asset: `{name}-{target}` (for example,
  `commit-x86_64-unknown-linux-gnu`)
- Checksums asset: `SHA256SUMS`

`SHA256SUMS` uses the standard `sha256sum` format:

```text
<hex>  <asset_name>
```

Each plugin directory has an `aish-plugin.toml` with the plugin `name` and
`version`. Pushing a matching tag, such as `commit-v0.1.0`, builds that workspace
package for the supported release targets and uploads the prebuilt binaries plus
`SHA256SUMS`.

When you run `aish plugin install <name>`, aish downloads the prebuilt binary for
the host target when a matching release asset exists. If no prebuilt asset is
available, it builds the plugin locally with Cargo.
