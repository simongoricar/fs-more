# Changelog
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).



## [Unreleased]



---

## [0.4.0] - 2024-03-13

### Changed
- Upgrade outdated dependencies.
- Improve documentation and contribution guide.
- Refactor internal test harness subcrates.
  
### Removed
- `TargetDirectoryRule` no longer publicly exposes the three methods that previously returned a bool indicating its existing target directory and overwriting behaviour.


## [0.3.0] - 2024-01-14

### Added
- Add `miette` feature flag that derives `miette::Diagnostic` on all error types,
  enabling covenient `wrap_err_with` and the like.



## [0.2.2] - 2024-01-13

### Fixed
- Fix `README.md` to correctly state library usage via `crates.io`, not Git.



## [0.2.1] - 2024-01-13

### Fixed
- Fix `license` field in `Cargo.toml`



## [0.2.0] - 2024-01-13

Initial development release with the API mostly stable.



[Unreleased]: https://github.com/simongoricar/fs-more/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/simongoricar/fs-more/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/simongoricar/fs-more/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/simongoricar/fs-more/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/simongoricar/fs-more/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/simongoricar/fs-more/compare/727e90a7ff9c70359fb9a4a5ebdf5e5e528f4708...v0.2.0
