# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-11-06

### Added
- Initial release of e3dc-mqtt-rs
- MQTT bridge for E3DC solar battery systems using RSCP protocol
- Real-time monitoring of system status, battery, grid, solar, and wallbox data
- TCP MQTT connection support (port 1883 or 8883 for TLS)
- Change detection to minimize MQTT traffic (only publish changed values)
- Configurable polling intervals for regular status and statistics data
- Comprehensive logging with configurable log levels
- MQTT Last Will and Testament for connection status monitoring
- Battery cell voltage and temperature monitoring
- Multiple wallbox support
- Daily statistics including:
  - Solar production
  - Grid consumption/feed-in
  - Battery charge/discharge
  - Wallbox consumption
  - Self-consumption and autarky rates
- Configuration via TOML file
- "Let it crash" error handling philosophy for reliability
- Comprehensive unit and integration test suite
- Support for custom MQTT topic prefix
- Credential redaction in debug output for security

### Security
- Passwords and encryption keys are redacted in all log output
- Custom Debug implementations for configuration structs

### Technical
- Type-safe error handling using `thiserror`
- All integer conversions use `try_into()` to prevent data corruption
- UTF-8 safe string operations
- Optimized RSCP library with read performance improvements
- Blocking I/O for simplicity and reliability
- Synchronous design with single background thread for MQTT event loop

[Unreleased]: https://github.com/isnogudus/e3dc-mqtt-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/isnogudus/e3dc-mqtt-rs/releases/tag/v0.1.0
