use crate::config::Config;
use crate::errors::MqttError;
use crate::mqtt::context::PublishContext;
use crate::mqtt::{BatteryData, DailyStatistics, DcbData, Status, SystemInfo};
use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use std::thread;
use std::time::Duration;

pub struct MqttPublisher {
    client: Client,
    root_topic: String,
}

macro_rules! publish_if_changed {
    ($context:expr, $src:ident , $old:ident, $field:ident) => {
        if $old.as_ref().map_or(true, |o| o.$field != $src.$field) {
            $context.publish(stringify!($field), &$src.$field)?;
        }
    };
}

impl MqttPublisher {
    pub fn new(config: &Config, device_id: String) -> Result<Self, MqttError> {
        let client_id = format!("e3dc-mqtt-rs-{}", device_id);

        let mut mqtt_options = if let Some(socket_path) = &config.mqtt.socket {
            // Unix domain socket connection
            tracing::info!("Using MQTT Unix socket: {}", socket_path);
            MqttOptions::new(client_id, socket_path, 0)
        } else {
            // TCP connection
            let host = config
                .mqtt
                .host
                .as_ref()
                .ok_or_else(|| MqttError::ClientError("MQTT host or socket must be configured".to_string()))?;

            tracing::info!("Using MQTT TCP connection: {}:{}", host, config.mqtt.port);
            MqttOptions::new(client_id, host, config.mqtt.port)
        };

        if !config.mqtt.username.is_empty() {
            mqtt_options.set_credentials(&config.mqtt.username, &config.mqtt.password);
        }

        mqtt_options.set_keep_alive(Duration::from_secs(60));

        // Set Last Will and Testament - publish "false" to online topic when connection is lost
        let online_topic = format!("{}/{}/online", config.mqtt.root, device_id);
        mqtt_options.set_last_will(rumqttc::LastWill {
            topic: online_topic.clone(),
            message: b"false".to_vec().into(),
            qos: QoS::AtLeastOnce,
            retain: true,
        });

        // Create blocking client (no async!)
        let (client, mut connection) = Client::new(mqtt_options, 10);

        // Spawn event loop in background thread (not tokio task!)
        thread::spawn(move || {
            for notification in connection.iter() {
                match notification {
                    Ok(Event::Incoming(Packet::ConnAck(_))) => {
                        tracing::info!("MQTT connected");
                    }
                    Ok(_) => {}
                    Err(e) => {
                        // On connection error, crash the process (let it crash philosophy)
                        tracing::error!("MQTT connection error: {:?}", e);
                        std::process::exit(1);
                    }
                }
            }
        });
        let root_topic = format!("{}/{}", config.mqtt.root, device_id);

        Ok(Self { client, root_topic })
    }

    pub fn context(&'_ self, topic: &str) -> PublishContext<'_> {
        let full_topic = if topic.is_empty() {
            self.root_topic.clone()
        } else {
            format!("{}/{}", self.root_topic, topic)
        };
        PublishContext::new(&self.client, full_topic)
    }

    pub fn publish_online_status(&self, online: bool) -> Result<(), MqttError> {
        let context = self.context("");
        context.publish("online", &online)
    }

    /// Publish system info as JSON
    pub fn publish_system_info(&self, info: &SystemInfo) -> Result<(), MqttError> {
        let context = self.context("");
        let json =
            serde_json::to_string(info).map_err(|error| MqttError::SerializationError { error })?;
        // Manual JSON formatting (no serde_json needed for simple structure)

        context.publish("info", &json)
    }

    /// Publish real-time status data
    /// Only publishes fields that have changed compared to prev_status
    pub fn publish_status(&self, status: &Status, old: Option<Status>) -> Result<(), MqttError> {
        let context = self.context("status");
        publish_if_changed!(context, status, old, time);
        publish_if_changed!(context, status, old, additional);
        publish_if_changed!(context, status, old, autarky);
        publish_if_changed!(context, status, old, battery_charge);
        publish_if_changed!(context, status, old, battery_discharge);
        publish_if_changed!(context, status, old, battery_consumption);
        publish_if_changed!(context, status, old, consumption_from_grid);
        publish_if_changed!(context, status, old, export_to_grid);
        publish_if_changed!(context, status, old, grid_production);
        publish_if_changed!(context, status, old, house_consumption);
        publish_if_changed!(context, status, old, self_consumption);
        publish_if_changed!(context, status, old, solar_production);
        publish_if_changed!(context, status, old, solar_production_excess);
        publish_if_changed!(context, status, old, state_of_charge);
        publish_if_changed!(context, status, old, wb_consumption);

        Ok(())
    }

    /// Publish daily statistics (status_sums)
    pub fn publish_daily_statistics(
        &self,
        stats: &DailyStatistics,
        old: Option<DailyStatistics>,
    ) -> Result<(), MqttError> {
        let context = self.context("status_sums");

        publish_if_changed!(context, stats, old, time);
        publish_if_changed!(context, stats, old, autarky_today);
        publish_if_changed!(context, stats, old, self_consumption_today);
        publish_if_changed!(context, stats, old, solar_production_today);
        publish_if_changed!(context, stats, old, house_consumption_today);
        publish_if_changed!(context, stats, old, battery_charge_today);
        publish_if_changed!(context, stats, old, battery_discharge_today);
        publish_if_changed!(context, stats, old, export_to_grid_today);
        publish_if_changed!(context, stats, old, consumption_from_grid_today);
        publish_if_changed!(context, stats, old, state_of_charge_today);
        publish_if_changed!(context, stats, old, start);
        publish_if_changed!(context, stats, old, timespan);

        Ok(())
    }

    pub fn publish_battery_data(
        &self,
        batteries: &[BatteryData],
        old: &[BatteryData],
    ) -> Result<(), MqttError> {
        for battery in batteries {
            let old_bat = old.iter().find(|b| b.index == battery.index);
            self.publish_battery_data_item(battery, old_bat)?;
        }
        Ok(())
    }
    /// Publish battery data (all fields, no change detection - kept for compatibility)
    fn publish_battery_data_item(
        &self,
        battery: &BatteryData,
        old: Option<&BatteryData>,
    ) -> Result<(), MqttError> {
        let context = self.context(format!("status/battery:{}", battery.index).as_str());
        publish_if_changed!(context, battery, old, time);
        publish_if_changed!(context, battery, old, asoc);
        publish_if_changed!(context, battery, old, charge_cycles);
        publish_if_changed!(context, battery, old, current);
        publish_if_changed!(context, battery, old, dcb_count);
        for dcb in &battery.dcbs {
            let old_dcb = old
                .as_ref()
                .and_then(|b| b.dcbs.iter().find(|d| d.index == dcb.index));
            self.publish_dcb_data(dcb, old_dcb, battery.index)?;
        }
        publish_if_changed!(context, battery, old, design_capacity);
        publish_if_changed!(context, battery, old, device_name);
        publish_if_changed!(context, battery, old, eod_voltage);
        publish_if_changed!(context, battery, old, error_code);
        publish_if_changed!(context, battery, old, fcc);
        publish_if_changed!(context, battery, old, index);
        publish_if_changed!(context, battery, old, max_battery_voltage);
        publish_if_changed!(context, battery, old, max_charge_current);
        publish_if_changed!(context, battery, old, max_discharge_current);
        publish_if_changed!(context, battery, old, max_dcb_cell_temp);
        publish_if_changed!(context, battery, old, min_dcb_cell_temp);
        publish_if_changed!(context, battery, old, module_voltage);
        publish_if_changed!(context, battery, old, rc);
        publish_if_changed!(context, battery, old, ready_for_shutdown);
        publish_if_changed!(context, battery, old, rsoc);
        publish_if_changed!(context, battery, old, rsoc_real);
        publish_if_changed!(context, battery, old, status_code);
        publish_if_changed!(context, battery, old, terminal_voltage);
        publish_if_changed!(context, battery, old, total_use_time);
        publish_if_changed!(context, battery, old, total_discharge_time);
        publish_if_changed!(context, battery, old, training_mode);
        publish_if_changed!(context, battery, old, usable_capacity);
        publish_if_changed!(context, battery, old, usable_remaining_capacity);

        Ok(())
    }

    fn publish_dcb_data(
        &self,
        data: &DcbData,
        old: Option<&DcbData>,
        bat_index: u64,
    ) -> Result<(), MqttError> {
        let context =
            self.context(format!("status/battery:{}/dcb:{}", bat_index, data.index).as_str());
        publish_if_changed!(context, data, old, current);
        publish_if_changed!(context, data, old, current_avg_30s);
        publish_if_changed!(context, data, old, cycle_count);
        publish_if_changed!(context, data, old, design_capacity);
        publish_if_changed!(context, data, old, design_voltage);
        publish_if_changed!(context, data, old, device_name);
        publish_if_changed!(context, data, old, end_of_discharge);
        publish_if_changed!(context, data, old, error);
        publish_if_changed!(context, data, old, full_charge_capacity);
        publish_if_changed!(context, data, old, fw_version);
        publish_if_changed!(context, data, old, manufacture_date);
        publish_if_changed!(context, data, old, manufacture_name);
        publish_if_changed!(context, data, old, max_charge_current);
        publish_if_changed!(context, data, old, max_charge_temperature);
        publish_if_changed!(context, data, old, max_charge_voltage);
        publish_if_changed!(context, data, old, max_discharge_current);
        publish_if_changed!(context, data, old, min_charge_temperature);
        publish_if_changed!(context, data, old, parallel_cell_count);
        publish_if_changed!(context, data, old, sensor_count);
        publish_if_changed!(context, data, old, series_cell_count);
        publish_if_changed!(context, data, old, pcb_version);
        publish_if_changed!(context, data, old, protocol_version);
        publish_if_changed!(context, data, old, remaining_capacity);
        publish_if_changed!(context, data, old, serial_no);
        publish_if_changed!(context, data, old, serial_code);
        publish_if_changed!(context, data, old, soc);
        publish_if_changed!(context, data, old, soh);
        publish_if_changed!(context, data, old, status);
        publish_if_changed!(context, data, old, temperatures);
        publish_if_changed!(context, data, old, voltage);
        publish_if_changed!(context, data, old, voltage_avg_30s);
        publish_if_changed!(context, data, old, voltages);
        publish_if_changed!(context, data, old, warning);

        Ok(())
    }
}
