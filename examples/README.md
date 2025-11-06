# Examples

This directory previously contained development examples that were used during the implementation of the E3DC MQTT bridge.

**All examples have been removed as the main application is now complete and fully functional.**

## Running the Application

To run the E3DC MQTT bridge:

```bash
# With default config.toml
cargo run --release

# With custom config file
cargo run --release -- --config /path/to/config.toml

# Show version
cargo run --release -- --version

# Show help
cargo run --release -- --help
```

## Configuration

See `config.toml.example` in the project root for configuration options.

## Log Levels

Set the log level in your `config.toml`:

```toml
[default]
log_level = "INFO"  # Options: TRACE, DEBUG, INFO, WARN, ERROR
```

- **INFO**: Shows startup messages, connections, and statistics every 5 minutes (recommended for production)
- **DEBUG**: Shows detailed status updates every 5 seconds (useful for development)
