# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/Unleash/yggdrasil-bindings/releases/tag/yggdrasilffi-v0.3.0) - 2025-09-09

### 🚀 Features
- expose get_state method in core binaries ([#295](https://github.com/unleash/unleash-edge/issues/295)) (by @kwasniew)
- move state ingestion entry point ([#213](https://github.com/unleash/unleash-edge/issues/213)) (by @sighphyre)
- add hydration method to ruby to receive delta updates ([#206](https://github.com/unleash/unleash-edge/issues/206)) (by @FredrikOseberg)
- *(java)* expose list_known_features and core version in java layer ([#197](https://github.com/unleash/unleash-edge/issues/197)) (by @sighphyre)
- list known features ([#157](https://github.com/unleash/unleash-edge/issues/157)) (by @sighphyre)
- bubble full error messages with details through to FFI layer ([#132](https://github.com/unleash/unleash-edge/issues/132)) (by @sighphyre)
- add feature enabled property to variant checks ([#98](https://github.com/unleash/unleash-edge/issues/98)) (by @sighphyre)
- *(yggdrasil)* Bubble up errors on take state ([#97](https://github.com/unleash/unleash-edge/issues/97)) (by @gastonfournier)
- add shouldemitimpressionevents to yggdrasil and .NET wrapper ([#75](https://github.com/unleash/unleash-edge/issues/75)) (by @daveleek)
- custom strategies in ruby and support in the core engine([#56](https://github.com/unleash/unleash-edge/issues/56)) (by @sighphyre)
- add dependent flags ([#49](https://github.com/unleash/unleash-edge/issues/49)) (by @thomasheartman)
- make ruby useful ([#43](https://github.com/unleash/unleash-edge/issues/43)) (by @sighphyre)
- add support for strategy variants ([#41](https://github.com/unleash/unleash-edge/issues/41)) (by @sighphyre)
- basic FFI language bindings (by @sighphyre)

### 🐛 Bug Fixes
- lock the ffi layer so that it can be safely accessed ([#262](https://github.com/unleash/unleash-edge/issues/262)) (by @sighphyre)
- undefined behaviour in metrics receiver when passed a boolean value with more than lsb set (by @sighphyre)
- move wasm engine to handle featureEnabled not feature_enabled ([#149](https://github.com/unleash/unleash-edge/issues/149)) (by @sighphyre)
- fix(core)/missing strategy variants no longer impacts other strategies ([#114](https://github.com/unleash/unleash-edge/issues/114)) (by @sighphyre)

### 💼 Other
- Release unleash-yggdrasil v0.13.0, safety bump 2 crates (by @chriswk)
- Release unleash-yggdrasil v0.8.0, safety bump 2 crates (by @sighphyre)

### 🚜 Refactor
- Yggdrasil FFI layer ([#47](https://github.com/unleash/unleash-edge/issues/47)) (by @nunogois)
- better public api for ffi consumers ([#42](https://github.com/unleash/unleash-edge/issues/42)) (by @sighphyre)

### ⚙️ Miscellaneous Tasks
- remove core project, brokken out from main yggdrasil project (by @sighphyre)
- internal refactor to free yggdrasil from random/sys clock, move to ahash and allow the internal functions to receive an EnrichedContext directly (by @sighphyre)
- javascript engine ([#226](https://github.com/unleash/unleash-edge/issues/226)) (by @nunogois)
- bump types to 0.15.6 (by @sjaanus)
- update to most recent unleash-types ([#191](https://github.com/unleash/unleash-edge/issues/191)) (by @chriswk)
- update unleash-types and chrono (by @chriswk)
- apply some lints and fixes ([#130](https://github.com/unleash/unleash-edge/issues/130)) (by @sighphyre)
- bump unleash types to 0.12 ([#124](https://github.com/unleash/unleash-edge/issues/124)) (by @sighphyre)
- *(core)* allow take_state to bubble warnings to caller ([#123](https://github.com/unleash/unleash-edge/issues/123)) (by @sighphyre)
- update types lib ([#118](https://github.com/unleash/unleash-edge/issues/118)) (by @sighphyre)
- expose known strategies ([#85](https://github.com/unleash/unleash-edge/issues/85)) (by @gastonfournier)
- *(java)* add spotless and migrate to build.gradle.kts ([#82](https://github.com/unleash/unleash-edge/issues/82)) (by @gastonfournier)
- apply cargo fmt ([#63](https://github.com/unleash/unleash-edge/issues/63)) (by @sighphyre)

### Contributors

* @sighphyre
* @kwasniew
* @nunogois
* @sjaanus
* @FredrikOseberg
* @chriswk
* @gastonfournier
* @daveleek
* @thomasheartman
