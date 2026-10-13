# Yggdrasil Node engine (wasm-bindgen)

Experimental WebAssembly bindings with synchronous Node.js loading.
This scaffolding exposes a single `add` function over the WASM boundary, with a
Vitest test and a Tinybench benchmark.

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
in `wasm/`. `yarn build:debug` uses the debug profile. Use release builds for benchmarks.
Set `WASM_BINDGEN` to use a CLI installed outside PATH.
The build also detects a CLI installed with `cargo install --root .tools`.

```js
const { add } = require('yggdrasil-engine')
const result = add(20, 22)
```

The CommonJS entry point and TypeScript declarations can be consumed by the Node SDK.
