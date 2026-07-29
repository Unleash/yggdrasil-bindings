# Python Bindings to Yggdrasil

Provides high level bindings to the Unleash Yggdrasil engine.

## Build and test
This project uses [poetry](https://python-poetry.org/).

Before you begin, you'll need to setup the native library. You'll need a Rust compiler. If you're on Windows, you'll need bash or just read the script and do the equivalent powershell steps.

``` sh
./build.sh
```

To run tests:

```poetry run pytest```

For local development, it can be convenient to have a shell to work in:

```poetry shell```

## Publish

Publishing is done through Github, with the `Publish Python` workflow. Ensure you've bumped `version` in `pyproject.toml` first.

Note that `__yggdrasil_core_version__` in `yggdrasil_engine/__init__.py` determines which version of the native libraries is resolved for the build; the build downloads prebuilt binaries from the corresponding `yggdrasilffi-v<version>` GitHub release and does not work against the Rust source code directly.