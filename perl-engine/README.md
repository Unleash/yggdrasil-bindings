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

## Dependencies

Perl modules:
- `FFI::Platypus`
- `JSON::PP` (core)

## Notes

The module loads native binaries from `lib/Yggdrasil/lib` by default.
You can override this with `YGGDRASIL_LIB_PATH`.
