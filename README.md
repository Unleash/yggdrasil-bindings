# Yggdrasil

![world tree image](worldtree.webp 'Title')

##### Bindings for the Unleash Yggdrasil SDK core.

This repository contains language bindings for the
[Unleash Yggdrasil core](https://github.com/Unleash/yggdrasil). The core SDK
domain logic lives in that project; this repository packages and exposes it for
Java, .NET, Ruby, and Python.

Language-specific build and test instructions live in each binding's README.

## Rust FFI

The shared Rust FFI layer can be built and tested from the repository root:

```
cargo test
```

To run the client specs, you'll first need to clone them:

```
git clone --depth 5 --branch v5.1.9 https://github.com/Unleash/client-specification.git client-specification
```

## Language Bindings

- [Java](java-engine/README.md)
- [.NET](dotnet-engine/README.md)
- [Ruby](ruby-engine/README.md)
- [Python](python-engine/README.md)

## Bumping Yggdrasil Core

Use the release helper when updating the bundled Yggdrasil core version across
the language bindings:

```
./scripts/bump-yggdrasil-core.py 0.21.3 --dry-run
./scripts/bump-yggdrasil-core.py 0.21.3
```

The script requires the target core version to be greater than the current
highest core pin. It updates the Rust core pins, the Java/Python/Ruby/.NET
native core metadata, and patch-bumps the Java, Python, Ruby, and .NET package
versions.
