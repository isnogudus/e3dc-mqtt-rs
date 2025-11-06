# e3dc-mqtt-rs

A high-performance MQTT bridge for E3DC solar battery systems, written in Rust.

## Features

- **Real-time Monitoring**: Publishes solar production, battery status, grid consumption, and more to MQTT
- **Change Detection**: Only publishes values that have changed, reducing MQTT traffic
- **Detailed Battery Data**: Complete DCB (DC Battery Controller) information including individual cell voltages and temperatures
- **Daily Statistics**: Autarky, self-consumption, energy production and consumption totals
- **Non-blocking**: Synchronous architecture with precise timing - no async complexity
- **Let it Crash**: Follows Erlang philosophy - crashes on errors for systemd/Docker to restart
- **Production Ready**: Type-safe, well-tested, handles edge cases properly

## Requirements

- Rust 1.70+ (2021 edition)
- E3DC solar battery system with RSCP access enabled
- MQTT broker (Mosquitto, etc.)
- E3DC portal credentials and RSCP key

## Installation

### From Source

**Note:** This project is currently only available from source, as it depends on the optimized `rscp` library which is not yet published on crates.io.

```bash
git clone https://github.com/isnogudus/e3dc-mqtt-rs.git
cd e3dc-mqtt-rs
cargo build --release
```

The binary will be in `target/release/e3dc-mqtt-rs`

## Configuration

Create a `config.toml` file (see `config.toml.example`):

```toml
[default]
log_level = "info"  # debug, info, warn, error

[e3dc]
host = "192.168.1.100"           # E3DC IP address
username = "your@email.com"      # E3DC portal username
password = "your-password"        # E3DC portal password
key = "your-rscp-key"            # RSCP encryption key from E3DC settings
interval = "5s"                   # Status update interval
statistic_update_interval = "60s" # Statistics update interval

[mqtt]
root = "e3dc"                     # MQTT root topic
username = "mqtt-user"            # MQTT username
password = "mqtt-password"        # MQTT password

# Connection: Choose either TCP or Unix socket
# Option 1: TCP connection (recommended for remote brokers)
host = "mqtt.example.com"         # MQTT broker hostname
port = 1883                       # MQTT broker port (1883 or 8883 for TLS)

# Option 2: Unix domain socket (recommended for local brokers)
# socket = "/run/mosquitto/mosquitto.sock"  # Takes precedence over host/port
```

### MQTT Connection Types

The bridge supports two connection types:

**Unix Domain Socket** (preferred for local broker):
```toml
[mqtt]
socket = "/run/mosquitto/mosquitto.sock"
username = "mqtt-user"
password = "mqtt-password"
root = "e3dc"
```

Benefits:
- Better performance (no TCP overhead)
- More secure (filesystem permissions)
- No port configuration needed
- Lower latency

**TCP Connection** (for remote brokers):
```toml
[mqtt]
host = "mqtt.example.com"
port = 1883  # or 8883 for TLS
username = "mqtt-user"
password = "mqtt-password"
root = "e3dc"
```

**Note:** If `socket` is specified, it takes precedence over `host`/`port`.
```

## Usage

### Run Directly

```bash
./e3dc-mqtt-rs --config config.toml
```

### Systemd Service

Create `/etc/systemd/system/e3dc-mqtt.service`:

```ini
[Unit]
Description=E3DC MQTT Bridge
After=network.target mosquitto.service

[Service]
Type=simple
User=e3dc
ExecStart=/usr/local/bin/e3dc-mqtt-rs --config /etc/e3dc-mqtt/config.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable e3dc-mqtt
sudo systemctl start e3dc-mqtt
```

### Docker

```dockerfile
FROM rust:1.70-slim as builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/e3dc-mqtt-rs /usr/local/bin/
ENTRYPOINT ["/usr/local/bin/e3dc-mqtt-rs"]
```

```bash
docker build -t e3dc-mqtt-rs .
docker run -v $(pwd)/config.toml:/config.toml e3dc-mqtt-rs --config /config.toml
```

## MQTT Topics

All topics are published under `{root}/{device-id}/` (e.g., `e3dc/S10E-12345678/`)

### System Info (retained)

Published once at startup:

- `info` - Full system information as JSON

### Real-time Status

Published every `interval` (default: 5 seconds), only if changed:

- `status/time` - Timestamp (RFC3339)
- `status/solar_production` - Solar production (W)
- `status/battery_charge` - Battery charging power (W)
- `status/battery_discharge` - Battery discharging power (W)
- `status/house_consumption` - House consumption (W)
- `status/grid_production` - Grid feed-in power (W)
- `status/consumption_from_grid` - Grid consumption (W)
- `status/state_of_charge` - Battery SOC (%)
- `status/autarky` - Current autarky (%)
- `status/self_consumption` - Current self-consumption (%)

### Daily Statistics

Published every `statistic_update_interval` (default: 60 seconds):

- `status_sums/autarky_today` - Daily autarky (%)
- `status_sums/self_consumption_today` - Daily self-consumption (%)
- `status_sums/solar_production_today` - Solar production today (Wh)
- `status_sums/house_consumption_today` - House consumption today (Wh)
- `status_sums/battery_charge_today` - Battery charged today (Wh)
- `status_sums/battery_discharge_today` - Battery discharged today (Wh)
- `status_sums/export_to_grid_today` - Grid feed-in today (Wh)
- `status_sums/consumption_from_grid_today` - Grid consumption today (Wh)

### Battery Details

Published for each battery (index 0, 1, ...) every `statistic_update_interval`:

- `status/battery:{index}/rsoc` - Real state of charge (%)
- `status/battery:{index}/voltage` - Battery voltage (V)
- `status/battery:{index}/current` - Battery current (A)
- `status/battery:{index}/temperature` - Battery temperature (°C)
- `status/battery:{index}/charge_cycles` - Total charge cycles
- `status/battery:{index}/device_name` - Battery model

#### DCB (DC Battery Controller) Data

For each DCB module (index 0, 1, ...) per battery:

- `status/battery:{bat}/dcb:{dcb}/voltages` - Cell voltages as JSON array (V)
- `status/battery:{bat}/dcb:{dcb}/temperatures` - Cell temperatures as JSON array (°C)
- `status/battery:{bat}/dcb:{dcb}/voltage` - Module voltage (V)
- `status/battery:{bat}/dcb:{dcb}/current` - Module current (A)
- `status/battery:{bat}/dcb:{dcb}/soc` - Module SOC (%)
- `status/battery:{bat}/dcb:{dcb}/soh` - Module state of health (%)
- `status/battery:{bat}/dcb:{dcb}/cycle_count` - Module charge cycles
- `status/battery:{bat}/dcb:{dcb}/serial_no` - Module serial number

## Architecture

### Design Philosophy

This project follows the **"Let it crash"** philosophy:

- Errors cause the process to exit (panic or return error from main)
- No retry logic, no error recovery
- Relies on external supervisor (systemd, Docker) to restart
- Simple, predictable behavior
- Easier to debug than complex recovery logic

### Code Structure

```
src/
├── main.rs              # Main loop and orchestration
├── lib.rs               # Library exports
├── config.rs            # TOML configuration parsing
├── errors.rs            # Error types (E3dcError, MqttError, BridgeError)
├── e3dc/
│   ├── mod.rs          # E3DC module exports
│   ├── client.rs       # RSCP protocol client
│   └── types.rs        # E3DC data structures
└── mqtt/
    ├── mod.rs          # MQTT module exports
    ├── publisher.rs    # MQTT publishing logic
    ├── context.rs      # Publishing abstraction
    └── types.rs        # MQTT data structures
```

### Key Technical Decisions

1. **Synchronous I/O**: No async/tokio - simpler code, easier debugging
2. **Change Detection**: Publish only changed values to reduce MQTT traffic
3. **Type Safety**: Strong typing throughout, no stringly-typed data
4. **Precise Timing**: Compensates for execution time to hit exact intervals
5. **Single Thread**: MQTT event loop runs in background thread, main loop is single-threaded

## Monitoring

### Check if Running

```bash
# Systemd
sudo systemctl status e3dc-mqtt

# Docker
docker ps | grep e3dc-mqtt
```

### View Logs

```bash
# Systemd
sudo journalctl -u e3dc-mqtt -f

# Docker
docker logs -f e3dc-mqtt-container
```

### Subscribe to MQTT Topics

```bash
# All topics
mosquitto_sub -h mqtt.example.com -u user -P pass -v -t "e3dc/#"

# Real-time status only
mosquitto_sub -h mqtt.example.com -u user -P pass -v -t "e3dc/+/status/#"

# Daily statistics only
mosquitto_sub -h mqtt.example.com -u user -P pass -v -t "e3dc/+/status_sums/#"
```

## Troubleshooting

### Connection Issues

**Problem**: "Failed to connect to E3DC"

**Solutions**:
- Verify E3DC IP address is correct
- Check that RSCP is enabled in E3DC web interface
- Verify username/password/key are correct
- Check network connectivity: `ping <e3dc-ip>`

### MQTT Issues

**Problem**: "MQTT connection error"

**Solutions**:
- Verify MQTT broker is running: `systemctl status mosquitto`
- Check broker address and port
- Verify MQTT credentials
- Check broker logs: `journalctl -u mosquitto`

### High CPU Usage

The application should use minimal CPU (< 1%). High usage indicates a problem:

- Check interval settings - too frequent updates
- Check E3DC connectivity - retries may be happening
- View logs for error messages

### Missing Data

**Problem**: Some MQTT topics not published

**Solutions**:
- Check logs for errors during battery/DCB data retrieval
- Verify battery is connected and online in E3DC portal
- Some values may not be supported by your E3DC model

## Development

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Check code
cargo check
cargo clippy
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings

# Check dependencies
cargo tree
```

## Performance

- **CPU**: < 1% on modern hardware
- **Memory**: ~5-10 MB RSS
- **Network**: Minimal - only changed values are published
- **Latency**: < 100ms from E3DC query to MQTT publish

## Security Considerations

- Store `config.toml` with restricted permissions (0600)
- Use TLS for MQTT connection (port 8883)
- Consider using Unix socket for local MQTT broker
- Credentials are never logged (redacted in debug output)
- No remote access - runs locally on your network

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Related Projects

This project builds upon and was inspired by the excellent E3DC community:

- **[python-e3dc](https://github.com/fsantini/python-e3dc)** - Python library for E3DC systems, foundational work for E3DC RSCP protocol reverse engineering
- **[pye3dc](https://github.com/vchrisb/pye3dc)** - Enhanced Python library with additional features and improvements
- **[rscp](https://github.com/isnogudus/rscp)** - Rust RSCP protocol implementation used by this project

Special thanks to the E3DC reverse engineering community for documenting the RSCP protocol and making these projects possible.

## Credits

- Uses [rscp](https://github.com/isnogudus/rscp) for E3DC communication
- Built with Rust and the excellent crates ecosystem
- Protocol knowledge from python-e3dc and pye3dc communities

## Contributing

Contributions welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and clippy
5. Submit a pull request

## Changelog

### v0.1.0 (Current)

- Initial release
- Real-time status monitoring
- Daily statistics
- Complete battery and DCB data
- Change detection for efficient MQTT traffic
- Production-ready error handling
