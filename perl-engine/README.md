# Perl Bindings to Yggdrasil

First-pass Perl FFI bindings for the Unleash Yggdrasil engine.

Implemented surface in this pass:
- `take_state`
- `is_enabled`
- `get_variant`
- custom strategy registration and evaluation
- metrics APIs (`get_metrics`, `count_toggle`, `count_variant`)
- impact metrics APIs (`define_*`, `inc_counter`, `set_gauge`, `observe_histogram`, `collect_impact_metrics`, `restore_impact_metrics`)

## Build

```sh
cd perl-engine
./build.sh
```

## Hello World

```sh
cd perl-engine
./bin/hello.pl
```

## Dependencies

Perl modules:
- `FFI::Platypus`
- `JSON::PP` (core)

Install dependencies with `cpanm`:

```sh
cpanm --notest FFI::Platypus
```

If you do not have `cpanm` yet, you should be able to install it via you OS package manager:

```sh
sudo apt-get install cpanminus
```

Or if not available, directly:

```sh
curl -L https://cpanmin.us | perl - App::cpanminus
```

## Run Tests

```sh
cd perl-engine
./build.sh
prove -I lib t
```

## Run Client Specification Tests

The spec files are read from `../client-specification/specifications` relative to `perl-engine/`.

```sh
cd perl-engine
prove -I lib t/specification.t
```

## Local Package Install (cpanm)

Use this when you want to test SDK integration against a locally packaged Perl bindings module.

Build and package:

```sh
cd perl-engine
./build.sh
perl Makefile.PL
make
make test
rm -f Yggdrasil-Engine-0.1.0.tar.gz
make dist
```

`MANIFEST` is checked in for repeatable packaging. If you add/remove source files, update `MANIFEST` accordingly before `make dist`.

Install the generated tarball into a local Perl lib directory:

```sh
cpanm -L /tmp/yggdrasil-perl-local Yggdrasil-Engine-0.1.0.tar.gz
```

Use that local package in your SDK build/test shell:

```sh
export PERL5LIB=/tmp/yggdrasil-perl-local/lib/perl5:$PERL5LIB
export PATH=/tmp/yggdrasil-perl-local/bin:$PATH
```

## Notes

The module loads native binaries from `lib/Yggdrasil/lib` by default.
You can override this with `YGGDRASIL_LIB_PATH`.
