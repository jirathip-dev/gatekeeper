# anti-slop-swift

The `swift/anti-slop` gate's tool: SwiftSyntax rules that reject low-evidence Swift. A port of
[sawfwair/anti-slop-swift](https://github.com/sawfwair/anti-slop-swift) at `259e1a32dd9a41e65478513ab3b902a1cb8036e0`
(MIT, `LICENSE`), itself a port of [dmmulroy/anti-slop](https://github.com/dmmulroy/anti-slop).

- The 19 shared rules always run; no config file or flag switches one off.
- The extra rules (`no-comments`, `sorted-imports`, `no-any`, `explicit-access`) run where a repo lists them in
  `[swift] extra`; the gate passes them as `--enable=`.
- `tools.yml` builds it once per version from `packs/swift/tools/anti-slop-swift.toml`; no gate run compiles
  swift-syntax. Each rule has a fixture case under `packs/swift/fixtures/anti-slop/`.

`anti-slop --list-rules` prints every rule.
