mod config;
mod e3dc;
mod errors;
mod mqtt;

use std::cmp::{max, min};

use chrono::{DateTime, Duration, TimeDelta, Utc};
use clap::Parser;
use config::Config;
use e3dc::E3dcClient;
use mqtt::MqttPublisher;
use tracing::{debug, error, info};

use crate::mqtt::DailyStatistics;

/// E3DC MQTT Bridge - Publishes E3DC solar system data to MQTT
#[derive(Parser)]
#[command(name = "e3dc-mqtt-rs")]
#[command(version)]
#[command(about = "E3DC MQTT Bridge - Publishes E3DC solar system data to MQTT", long_about = None)]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

/// Round timestamp to next modulo seconds (Python-style precise timing)
/// Example: round_to_next_modulo_seconds(12.3, 5.0) -> 15.0
fn next_interval(time: DateTime<Utc>, interval: Duration) -> DateTime<Utc> {
    let duration_since_last_interval = Duration::seconds(time.timestamp() % interval.num_seconds());
    time - duration_since_last_interval + interval
}

fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Load configuration first (to get log level)
    let config_path = cli.config;
    let config = Config::from_file(&config_path)?;

    // Initialize tracing with log level from config
    let app_log_level = config.default.log_level.as_str();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(format!("e3dc_mqtt_rs={}", app_log_level).parse()?)
                .add_directive("rscp=warn".parse()?), // Only show warnings/errors from rscp
        )
        .init();

    let interval = Duration::from_std(config.e3dc.interval)?;
    let statistic_interval = Duration::from_std(config.e3dc.statistic_update_interval)?;

    info!("Configuration loaded from: {}", config_path);
    info!("Log level: {}", config.default.log_level);
    debug!("Debug logging is enabled");

    info!("Configuration loaded successfully!");
    info!("  E3DC Host: {}", config.e3dc.host);
    info!("  MQTT Root: {}", config.mqtt.root);
    info!("  Interval: {:?}", interval);
    info!("  Statistics Interval: {:?}", statistic_interval);

    // Create E3DC client
    info!("Creating E3DC client...");
    let mut e3dc_client = E3dcClient::new(
        config.e3dc.host.clone(),
        config.e3dc.key.clone(),
        config.e3dc.username.clone(),
        config.e3dc.password.clone(),
    )?;

    let batteries = e3dc_client.batteries().clone();

    let system_info = e3dc_client.get_system_info()?;
    let device_id = format!("{}-{}", system_info.model, system_info.serial_number);
    info!("Device ID: {}", device_id);

    // Query batteries at startup to know how many we have and their DCB counts
    info!("Querying batteries...");
    info!("Found {} battery/batteries", batteries.len());
    for battery in batteries.iter() {
        info!(
            "  Battery {}: {} DCB modules",
            battery.index, battery.dcb_count
        );
    }

    // Create MQTT publisher (blocking)
    info!("Creating MQTT publisher...");
    let mqtt_publisher = MqttPublisher::new(&config, device_id.clone())?;
    info!("✓ MQTT publisher created successfully!");

    // Give MQTT a moment to connect
    std::thread::sleep(Duration::milliseconds(500).to_std()?);

    // Publish online status
    mqtt_publisher.publish_online_status(true)?;
    info!("✓ Published online status");

    // Publish initial system info
    mqtt_publisher.publish_system_info(&mqtt::SystemInfo::from_e3dc(&system_info))?;
    info!("✓ Published system info");

    // Python-style timing: track next loop times
    let mut next_loop = Utc::now();
    let mut next_statistic_loop = Utc::now();

    let mut last_status: Option<mqtt::Status> = None;
    let mut last_battery_data: Vec<mqtt::BatteryData> = Vec::new();
    let mut last_daily_stats: Option<DailyStatistics> = None;
    info!("Starting main loop...");

    loop {
        let now = Utc::now();
        if now >= next_loop {
            next_loop = next_interval(now, interval);

            // Get and publish current status (always)
            let status = e3dc_client.get_status()?;
            // Publish to MQTT (per-field change detection inside publish_status)
            let mqtt_status = mqtt::Status::from_e3dc(&status);
            if let Err(e) = mqtt_publisher.publish_status(&mqtt_status, last_status) {
                error!("Failed to publish status: {:?}", e);
                // Let it crash on MQTT errors
                return Err(e.into());
            }

            debug!(
                "Status: Solar={:.0}W Battery={:.0}W Grid={:.0}W Home={:.0}W SOC={:.1}%",
                status.power_pv,
                status.power_battery,
                status.power_grid,
                status.power_home,
                status.battery_soc
            );
            last_status = Some(mqtt_status);
        }

        // Get and publish statistics (only when interval has elapsed)
        if now >= next_statistic_loop {
            next_statistic_loop = next_interval(now, statistic_interval);

            // Publish daily statistics
            let interval = TimeDelta::from_std(config.e3dc.statistic_update_interval)?;
            let e3dc_stats = e3dc_client.get_daily_statistics(interval)?;
            let stats = mqtt::DailyStatistics::from_e3dc(&e3dc_stats);
            if let Err(e) = mqtt_publisher.publish_daily_statistics(&stats, last_daily_stats) {
                error!("Failed to publish daily statistics: {:?}", e);
                return Err(e.into());
            }
            info!(
                "Statistics: Autarky={:.1}% SelfCons={:.1}% Solar={}Wh Consumption={}Wh",
                e3dc_stats.autarky,
                e3dc_stats.consumed_production,
                e3dc_stats.solar_production,
                e3dc_stats.consumption
            );

            last_daily_stats = Some(stats);

            // Publish battery data for all known batteries with change detection
            // Battery data now includes DCBs, much simpler!
            let battery_data = e3dc_client.get_battery_data()?;
            let bat_data: Vec<mqtt::BatteryData> = battery_data
                .iter()
                .map(mqtt::BatteryData::from_e3dc)
                .collect();
            mqtt_publisher.publish_battery_data(&bat_data, &last_battery_data)?;

            for battery in &bat_data {
                debug!(
                    "Battery {}: SOC={:.1}%, {} DCBs with {} cells each",
                    battery.index,
                    battery.rsoc_real,
                    battery.dcb_count,
                    battery.dcbs.first().map(|d| d.voltages.len()).unwrap_or(0)
                );
            }

            last_battery_data = bat_data;
        }

        // Python-style sleep: compensate for execution time
        let sleep_duration = max(
            min(next_loop, next_statistic_loop) - Utc::now(),
            Duration::milliseconds(100),
        );

        std::thread::sleep(
            sleep_duration
                .to_std()
                .expect("Sleep duration invalid - this is a bug in timing calculation"),
        );
    }
}
