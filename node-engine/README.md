# Yggdrasil Node engine (wasm-bindgen)

Experimental WebAssembly bindings with synchronous Node.js loading.
Context crosses the WASM boundary as JSON and is deserialized into owned strings
and maps before constructing an enriched engine context.

Requires Rust, Node.js, Yarn and a matching wasm-bindgen CLI:

Yarn is managed by Corepack using the version pinned in `package.json`; its CLI
is not committed to this repository. Run `corepack enable` once to enable Yarn.

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
yarn install
yarn build
yarn test
yarn bench
```

`yarn build` compiles an optimized WASM module and generates the CommonJS loader
as `wasm/wasm-interface.js`, then compiles `src/unleash-engine.ts` to CommonJS JavaScript and TypeScript
declarations in `lib/`. `yarn build:debug` uses the debug profile. Use release builds for benchmarks.
After wrapper-only changes, `yarn build:wrapper` regenerates the JS and declarations
without rebuilding Rust. Both `lib/` and `wasm/` are generated; edit `src/unleash-engine.ts`
to add wrapper methods and their types.

Response types are derived from the Rust structs with Tsify and emitted into
`wasm/wasm-interface.d.ts`. The public TypeScript entry point re-exports those
types and infers method return types from the generated WASM declarations.
Rebuild with `yarn build` after changing a Rust response type.
Set `WASM_BINDGEN` to use a CLI installed outside PATH.
The build also detects a CLI installed with `cargo install --root .tools`.

```js
const { UnleashEngine } = require('yggdrasil-engine')
const engine = new UnleashEngine()
engine.takeState(JSON.stringify({ version: 1, features: [] }))
const result = engine.isEnabled('demo-feature', { userId: 'user-1' })
engine.destroy()
```

`takeState` returns warnings or `null`. `isEnabled` returns an object with optional
`enabled` and boolean `impressionData`. The wrapper releases WASM-owned state on
`destroy()`, which is idempotent. Calls after destruction throw an error.
