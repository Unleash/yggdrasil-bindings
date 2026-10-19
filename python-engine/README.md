# Python Bindings to Yggdrasil

Provides high level bindings to the Unleash Yggdrasil engine.

Implements the engine methods the Python SDK uses.

- `src/`: the Rust part, compiled into the Python module `yggdrasil_engine.yggdrasil_native`
  - `lib.rs`: the Python glue around the core engine
- `python/yggdrasil_engine/`: the Python package
- `tests/`: python tests
- `scripts/smoke_test.py`: checks an installed wheel; the publish workflow runs it on every platform

## Test locally

Rust and Python 3.10 or newer is needed. Run everything from this folder (`python-engine/`).

The Python tests include the client specification, which they read from the repository root:

```sh
git clone --depth 1 --branch v6.1.0 https://github.com/Unleash/client-specification.git ../client-specification
```

### Rust

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

### Python


```sh
python3 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install --group dev
```

Then build the engine, with one of the two commands below.

Both turn on the `test-helpers` cargo feature. It adds test-only functions to the module, which
`tests/test_context_reader.py` needs. The published wheel is built without it.

**While developing: `maturin develop`**

```sh
maturin develop --features test-helpers
```

- compiles the Rust code and puts the module into `python/yggdrasil_engine/`; the virtual
  environment uses this folder directly, so Python changes apply without rebuilding
- after changing Rust code, run it again
- debug build by default (faster to compile, slower to run); add `--release` for an optimised build

**Like CI: install the package**

```sh
MATURIN_PEP517_ARGS="--features test-helpers" python -m pip install .
```

- builds the package the way the wheel is built (release mode) and installs it into the virtual
  environment, so the tests run against the installed package
- after changing any code, Rust or Python, run it again

Then run the checks:

```sh
pytest
ruff format --check .
ruff check .
```

### Context reader tests

`tests/test_context_reader.py` reads each context both with our reader and with `unleash-types'
parsing of `json.dumps(context)`, and expects the same result. Needs the `test-helpers` feature.

To run one case, give its full test ID (the case name goes in the brackets):

```sh
pytest -v "tests/test_context_reader.py::test_context_reader_matches_unleash_types[leftover string overrides properties]"
```

To run several cases, `-k` matches words in the case names, e.g. every case with "leftover":

```sh
pytest tests/test_context_reader.py -v -k leftover
```

When a case fails, `-vv` prints the full diff of the two contexts, and `--showlocals` (`-l`) also
prints the test's local variables:

```sh
pytest tests/test_context_reader.py -vv -l -k leftover
```

## Publish

1. Bump `version` in `pyproject.toml`
2. Run the manual `Publish Python` workflow (`.github/workflows/publish-python.yml`)

It builds one wheel per platform (Linux glibc and musl, macOS, Windows; x86_64 and arm64) and an
sdist, installs each on a machine of its platform, runs `scripts/smoke_test.py`, and uploads to PyPI
only if every test passed. With `dry_run` (the default) it builds and tests without uploading.
