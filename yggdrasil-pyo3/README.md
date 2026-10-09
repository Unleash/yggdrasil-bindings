# yggdrasil-pyo3

Python engine for evaluating Unleash feature flags, built with [PyO3](https://pyo3.rs/) and
[maturin](https://www.maturin.rs/). In development: not published and not used by the Python SDK
yet. 

So far it only `take_state` and `is_enabled` are implemented.

- `src/`: the Rust part, compiled into the Python module `yggdrasil_pyo3.yggdrasil_native`
  - `engine.rs`: the engine logic (plain Rust, no PyO3)
  - `lib.rs`: the Python glue
- `python/yggdrasil_pyo3/`: the Python package
- `tests/`: python tests

## Test locally

Rust and Python 3.9 or newer is needed. Run everything from this folder (`yggdrasil-pyo3/`).

The Python tests include the client specification, which they read from the repository root:

```sh
git clone --depth 1 --branch v6.1.0 https://github.com/Unleash/client-specification.git ../client-specification
```

### Rust

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

### Python


```sh
python3 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install --group dev
```

Then build the package in one of two ways.

**Install the built package.** 

maturin compiles
the Rust module and builds the package, and pip installs it into the virtual environment. Tests run
against the package.

```sh
python -m pip install .
```

**For development: `maturin develop`.** 

builds the Rust module into `python/yggdrasil_pyo3/` and points the virtual environment at this folder

run it again after changing Rust code

builds in debug mode by default; add `--release` for an optimised build:

```sh
maturin develop
```

run the checks:

```sh
pytest
ruff format --check .
ruff check .
```
