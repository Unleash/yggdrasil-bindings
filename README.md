# Yggdrasil

![world tree image](worldtree.webp 'Title')

##### Bindings for the Unleash Yggdrasil SDK core.

This repository contains language bindings for the
[Unleash Yggdrasil core](https://github.com/Unleash/yggdrasil). The core SDK
domain logic lives in that project; this repository wraps it in a Rust FFI layer
and a pair of WebAssembly modules, then packages those for Java, .NET, Ruby,
Python, JavaScript, Go, and PHP.

Language-specific build and test instructions live in each binding's README.

## Repository layout

| Path                                    | What it is                                                                     |
| --------------------------------------- | ------------------------------------------------------------------------------ |
| `yggdrasilffi/`                         | Rust `cdylib` exposing the core over a C ABI. The only Cargo workspace member. |
| `pure-wasm/`                            | Standalone `wasm32-unknown-unknown` module. No WASI required.                  |
| `yggdrasilwasm/`                        | `wasm-bindgen` crate, published to npm as `@unleash/yggdrasil-wasm`.           |
| `flat-buffer-defs/`                     | `enabled-message.fbs`, the FlatBuffers schema shared across the boundary.      |
| `*-engine/`                             | The seven language bindings. See [Language bindings](#language-bindings).      |
| `scripts/`                              | `bump-yggdrasil-core.py`, the release helper.                                  |
| `test-data/`, `yggdrasilffi/testfiles/` | Test fixtures compiled into the Rust tests.                                    |

> [!NOTE]
> `pure-wasm` and `yggdrasilwasm` are Cargo packages but are **not** workspace
> members — the root `Cargo.toml` lists only `yggdrasilffi`. A root `cargo build`
> or `cargo test` skips them; build them from their own directories.

## Getting started

You need a Rust toolchain for everything that touches the FFI, plus the toolchain
for whichever binding you're working on.

The repository ships a [devenv](https://devenv.sh/) shell (`devenv.nix`) that
provides Rust and the GitHub CLI, wired up through direnv via `.envrc`.
`mise.toml` pins Java to 21.

| Binding    | Toolchain                                            |
| ---------- | ---------------------------------------------------- |
| Java       | JDK 21, Gradle wrapper (`./gradlew`)                 |
| .NET       | `dotnet` SDK; targets `netstandard2.0` and `net6.0`  |
| Ruby       | Ruby 3.2, Bundler, RSpec                             |
| Python     | [Poetry](https://python-poetry.org/) (not pip or uv) |
| JavaScript | [Bun](https://bun.sh/)                               |
| Go         | Go 1.18+, cgo                                        |
| PHP        | Composer, PHPUnit, the FFI extension                 |

## Client specification fixtures

The [Unleash client specification](https://github.com/Unleash/client-specification)
drives the conformance test suites. It is not vendored — you clone it yourself.

**The root `cargo test` does not need it.** The Rust tests embed their fixtures
from `yggdrasilffi/testfiles/`. Only the language binding test suites read the
specification.

Every binding except Java resolves `client-specification/specifications` relative
to the repository root, so clone it once, here:

```
git clone --depth 5 --branch v6.1.0 https://github.com/Unleash/client-specification.git client-specification
```

`/client-specification` is already in `.gitignore`.

Java is the exception: its `fetchClientSpecification` Gradle task downloads and
unpacks its own copy, pinned by `clientSpecificationVersion` in
`java-engine/gradle.properties`. Running `./gradlew test` needs no manual clone.

## Rust FFI

The shared Rust FFI layer builds and tests from the repository root:

```
cargo test
cargo build --release
```

Regenerating the C header (needed by the Go and PHP bindings) and the FlatBuffers
bindings is documented in [`yggdrasilffi/README.md`](yggdrasilffi/README.md).

## Language bindings

### Published

These have both a build workflow and a publish workflow.

| Binding                                   | Package                          | Registry      | Version      |
| ----------------------------------------- | -------------------------------- | ------------- | ------------ |
| [Java](java-engine/README.md)             | `io.getunleash:yggdrasil-engine` | Maven Central | 1.0.2        |
| [.NET](dotnet-engine/README.md)           | `Unleash.Yggdrasil`              | NuGet         | 1.3.1        |
| [Ruby](ruby-engine/README.md)             | `yggdrasil-engine`               | RubyGems      | 1.3.1        |
| [Python](python-engine/README.md)         | `yggdrasil-engine`               | PyPI          | 1.3.1        |
| [JavaScript](javascript-engine/README.md) | `@unleash/yggdrasil-engine`      | npm           | 0.0.1-beta.2 |

JavaScript is the odd one out: it binds to the WebAssembly build via the npm
package `@unleash/yggdrasil-wasm`, not to the FFI, and is the only binding whose
CI does not compile the Rust FFI.

### In-repo, unsupported

These exist and have READMEs, but nothing builds, tests, or releases them — there
are no CI or publish workflows for any of them, and `bump-yggdrasil-core.py` does
not touch them. Treat them as unmaintained until that changes.

| Binding                                  | Note                                                                                      |
| ---------------------------------------- | ----------------------------------------------------------------------------------------- |
| [Go](go-engine/README.md)                | `go.mod` still declares the stale module path `github.com/sighphyre/yggdrasil/go-engine`. |
| [PHP](php-engine/README.md)              | Consumes the C ABI via PHP's FFI extension.                                               |
| [pure-wasm](pure-wasm/README.md)         | Standalone WASM module; `publish = false`.                                                |
| [yggdrasilwasm](yggdrasilwasm/README.md) | Published to npm manually; `publish = false` for crates.io.                               |

## How the pieces fit together

Worth internalising before touching the release process:

The Rust FFI is built and released **first**, as a GitHub release carrying
prebuilt binaries for nine targets. Each language binding pins an FFI version in
its own metadata file, and at publish time **downloads those prebuilt binaries**
rather than compiling any Rust. That is why every binding carries a
`yggdrasilCoreVersion`-style field, and why the bump script writes the *FFI*
version into those fields rather than the Yggdrasil core version.

Local development is the exception — `build.sh` in each binding compiles the FFI
from source and drops it where the tests expect it.

But that's not necessarily the case for a given patch - these are intentionally cleaved 
to allow the binding versions to evolve without needing a binary version increment. 
For example, if you wanted to say... tweak the Python bindings public API to collapse
a few functions into a single call, you would need to version the Python bindings as 
a new major version but touching the FFI layers would be silly and unnecessary.

## Bumping the Yggdrasil crate

Use the release helper when updating the dependency on the Yggdrasil crate:

```
./scripts/bump-yggdrasil-core.py 0.21.3 --dry-run
./scripts/bump-yggdrasil-core.py 0.21.3
```

`--dry-run` prints the plan and writes nothing. The target version must be
strictly greater than the current highest pin, so re-running with the same
version fails.

A run makes four kinds of change:

1. Sets the `unleash-yggdrasil` pin to the given version in **three** manifests:
   `yggdrasilffi/Cargo.toml`, `pure-wasm/Cargo.toml`, `yggdrasilwasm/Cargo.toml`.
2. **Patch-bumps `yggdrasilffi`'s own package version** in `yggdrasilffi/Cargo.toml`.
3. Points the four FFI artifact fields at that new FFI version — `yggdrasilCoreVersion`
   in `java-engine/gradle.properties`, `__yggdrasil_core_version__` in
   `python-engine/yggdrasil_engine/__init__.py`, `yggdrasil_core_version` in the
   Ruby gemspec metadata, and `<YggdrasilCoreVersion>` in the .NET csproj.
4. Patch-bumps the Java, Python, Ruby and .NET package versions.

It does not touch `javascript-engine`, `go-engine` or `php-engine`.

## Releasing

Nothing releases on a tag push. **Every publish workflow is `workflow_dispatch`
only**, and the workflows *create* tags rather than react to them. Releasing is
two stages, and the FFI always goes first.

### 1. Release the FFI

Run the `Release FFI and Upload Binaries` workflow
(`.github/workflows/release-new-ffi-version.yaml`).

It reads `version` from the `[package]` section of `yggdrasilffi/Cargo.toml`,
pushes the tag `yggdrasilffi-v<version>`, generates a changelog with `git-cliff`,
cross-builds nine targets, and uploads the binaries as release assets.

You only need to bump `yggdrasilffi/Cargo.toml` by hand when releasing the FFI
*without* a core bump. If you ran `bump-yggdrasil-core.py`, the bump is already
in the commit — bumping again would skip a version.

### 2. Publish the bindings

Dispatch the relevant workflow: `Publish Java`, `Publish .NET`, `Publish Ruby`,
`Publish Python`, or `Publish JavaScript`. Each reads its pinned FFI version, runs
`gh release download "yggdrasilffi-v$CORE_VERSION"` to fetch the binaries from
stage 1, publishes to its registry, and tags the repo `<lang>-engine-v<version>`.

Two exceptions worth knowing:

- **JavaScript** takes its version as a `workflow_dispatch` input rather than
  reading it from a file, and commits the bump back to the repo.
- **Ruby** publishes through RubyGems trusted publishing (OIDC). The others
  authenticate with API token secrets.

## CI

| Workflow                       | File                       | Triggers on                                                          |
| ------------------------------ | -------------------------- | -------------------------------------------------------------------- |
| Clippy analysis (Code Quality) | `sarif-and-test.yaml`      | All pushes/PRs to `main` except `java-engine/**`, plus a weekly cron |
| Java build                     | `build-java.yaml`          | `java-engine` Java sources and `build.gradle.kts`                    |
| .NET Build                     | `build-dotnet.yaml`        | `dotnet-engine/**`                                                   |
| Ruby build                     | `build-ruby.yml`           | `ruby-engine/**`, `yggdrasilffi/**`, `Cargo.toml`                    |
| Python build                   | `build-python.yml`         | `python-engine/**`, `yggdrasilffi/**`, `Cargo.toml`                  |
| Build JavaScript               | `build-javascript.yml`     | `javascript-engine/**`                                               |
| Build Rust binaries            | `build-rust-binaries.yaml` | Manual dispatch; the release build matrix without the release        |

Despite its name, `Clippy analysis (Code Quality)` is the **main Rust test
workflow** — it runs `cargo fmt --check`, `cargo clippy` with SARIF upload, and
`cargo test --all-features`.

There is no CI for `go-engine`, `php-engine`, `pure-wasm` or `yggdrasilwasm`.
