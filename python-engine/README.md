# Python Bindings to Yggdrasil

Provides high level bindings to the Unleash Yggdrasil engine.

## Build and test
This project uses [poetry](https://python-poetry.org/).

Before you begin, you'll need to setup the native library. You'll need a Rust compiler. If you're on Windows, you'll need bash or just read the script and do the equivalent powershell steps.

``` sh
./build-and-vendor-ffi.sh
```

To run tests:

```poetry run pytest```

For local development, it can be convenient to have a shell to work in. On Poetry 2.0 and later this
lives in the [shell plugin](https://github.com/python-poetry/poetry-plugin-shell):

```poetry shell```

## Lint and format

``` sh
poetry install                      # install dev dependencies

poetry run ruff format .            # apply formatting
poetry run ruff check --fix .       # apply lint autofixes

poetry run ruff format --check .    # verify formatting
poetry run ruff check .             # verify lint
poetry run basedpyright             # verify types

poetry run basedpyright --writebaseline   # re-baseline after formatting or refactors
```

## Publish

Publishing is done through Github, with the `Publish Python` workflow. Ensure you've bumped `version` in `pyproject.toml` first.
