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
