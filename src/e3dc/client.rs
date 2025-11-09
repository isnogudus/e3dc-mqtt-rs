//! E3DC client wrapper
//!
//! High-level interface to E3DC RSCP protocol

use std::{any::Any, collections::HashMap};

use super::types::*;
use crate::errors::E3dcError;
use chrono::{DateTime, Duration, Timelike, Utc};
use rscp::{
    tags::{BAT, DB, EMS, INFO},
    Client, Frame, GetItem, Item,
};
use tracing::info;

/// Minimum valid cell temperature in Celsius.
/// E3DC firmware returns 0.0 for missing/invalid sensors.
const MIN_VALID_CELL_TEMP_C: f64 = 10.0;

fn any_to_items(data: &Option<Box<dyn Any>>) -> Result<Vec<&Item>, E3dcError> {
    if let Some(value) = data {
        return match value.downcast_ref::<Vec<Item>>() {
            Some(v) => Ok(v.iter().collect()),
            None => Ok(Vec::new()),
        };
    }
    Ok(Vec::new())
}

fn any_to_string(value: &Box<dyn Any>) -> Result<String, E3dcError> {
    if let Some(v) = value.downcast_ref::<String>().cloned() {
        return Ok(v);
    }
    if let Some(&v) = value.downcast_ref::<bool>() {
        return Ok(if v {
            "true".to_string()
        } else {
            "false".to_string()
        });
    }
    if let Some(&v) = value.downcast_ref::<i8>() {
        return Ok(v.to_string());
    }
    if let Some(&v) = value.downcast_ref::<u8>() {
        return Ok(v.to_string());
    }
    if let Some(&v) = value.downcast_ref::<i16>() {
        return Ok(v.to_string());
    }
    if let Some(&v) = value.downcast_ref::<u16>() {
        return Ok(v.to_string());
    }
    if let Some(&v) = value.downcast_ref::<i32>() {
        return Ok(v.to_string());
    }
    if let Some(&v) = value.downcast_ref::<u32>() {
        return Ok(v.to_string());
    }
    if let Some(&v) = value.downcast_ref::<i64>() {
        return Ok(v.to_string());
    }
    if let Some(&v) = value.downcast_ref::<u64>() {
        return Ok(v.to_string());
    }
    if let Some(&v) = value.downcast_ref::<f32>() {
        return Ok(v.to_string());
    }
    if let Some(&v) = value.downcast_ref::<f64>() {
        return Ok(v.to_string());
    }
    Err(E3dcError::Type(format!(
        "Cannot convert {:?} to string",
        (**value).type_id()
    )))
}

fn any_to_f64(value: &Box<dyn Any>) -> Result<f64, E3dcError> {
    if let Some(&v) = value.downcast_ref::<bool>() {
        return Ok(if v { 1.0 } else { 0.0 });
    }
    if let Some(&v) = value.downcast_ref::<i8>() {
        return Ok(v as f64);
    }
    if let Some(&v) = value.downcast_ref::<u8>() {
        return Ok(v as f64);
    }
    if let Some(&v) = value.downcast_ref::<i16>() {
        return Ok(v as f64);
    }
    if let Some(&v) = value.downcast_ref::<u16>() {
        return Ok(v as f64);
    }
    if let Some(&v) = value.downcast_ref::<i32>() {
        return Ok(v as f64);
    }
    if let Some(&v) = value.downcast_ref::<u32>() {
        return Ok(v as f64);
    }
    if let Some(&v) = value.downcast_ref::<i64>() {
        return Ok(v as f64);
    }
    if let Some(&v) = value.downcast_ref::<u64>() {
        return Ok(v as f64);
    }
    if let Some(&v) = value.downcast_ref::<f32>() {
        return Ok(v as f64);
    }
    if let Some(&v) = value.downcast_ref::<f64>() {
        return Ok(v);
    }
    Err(E3dcError::Type(format!(
        "Cannot convert {:?} to f64",
        (**value).type_id()
    )))
}

fn any_to_u64(value: &Box<dyn Any>) -> Result<u64, E3dcError> {
    if let Some(&v) = value.downcast_ref::<bool>() {
        return Ok(if v { 1 } else { 0 });
    }
    if let Some(&v) = value.downcast_ref::<i8>() {
        return v
            .try_into()
            .map_err(|_| E3dcError::Type(format!("Negative i8 {} cannot convert to u64", v)));
    }
    if let Some(&v) = value.downcast_ref::<u8>() {
        return Ok(v as u64);
    }
    if let Some(&v) = value.downcast_ref::<i16>() {
        return v
            .try_into()
            .map_err(|_| E3dcError::Type(format!("Negative i16 {} cannot convert to u64", v)));
    }
    if let Some(&v) = value.downcast_ref::<u16>() {
        return Ok(v as u64);
    }
    if let Some(&v) = value.downcast_ref::<i32>() {
        return v
            .try_into()
            .map_err(|_| E3dcError::Type(format!("Negative i32 {} cannot convert to u64", v)));
    }
    if let Some(&v) = value.downcast_ref::<u32>() {
        return Ok(v as u64);
    }
    if let Some(&v) = value.downcast_ref::<i64>() {
        return v
            .try_into()
            .map_err(|_| E3dcError::Type(format!("Negative i64 {} cannot convert to u64", v)));
    }
    if let Some(&v) = value.downcast_ref::<u64>() {
        return Ok(v);
    }
    if let Some(&v) = value.downcast_ref::<f32>() {
        if !v.is_finite() || v < 0.0 || v > u64::MAX as f32 {
            return Err(E3dcError::Type(format!("Invalid f32 {} for u64", v)));
        }
        return Ok(v as u64);
    }
    if let Some(&v) = value.downcast_ref::<f64>() {
        if !v.is_finite() || v < 0.0 || v > u64::MAX as f64 {
            return Err(E3dcError::Type(format!("Invalid f64 {} for u64", v)));
        }
        return Ok(v as u64);
    }
    Err(E3dcError::Type(format!(
        "Cannot convert {:?} to u64",
        (**value).type_id()
    )))
}

fn any_to_bool(value: &Box<dyn Any>) -> Result<bool, E3dcError> {
    const EPSILON32: f32 = 1e-10;
    const EPSILON64: f64 = 1e-10;

    if let Some(&v) = value.downcast_ref::<bool>() {
        return Ok(v);
    }
    if let Some(&v) = value.downcast_ref::<i8>() {
        return Ok(v != 0);
    }
    if let Some(&v) = value.downcast_ref::<u8>() {
        return Ok(v != 0);
    }
    if let Some(&v) = value.downcast_ref::<i16>() {
        return Ok(v != 0);
    }
    if let Some(&v) = value.downcast_ref::<u16>() {
        return Ok(v != 0);
    }
    if let Some(&v) = value.downcast_ref::<i32>() {
        return Ok(v != 0);
    }
    if let Some(&v) = value.downcast_ref::<u32>() {
        return Ok(v != 0);
    }
    if let Some(&v) = value.downcast_ref::<i64>() {
        return Ok(v != 0);
    }
    if let Some(&v) = value.downcast_ref::<u64>() {
        return Ok(v != 0);
    }
    if let Some(&v) = value.downcast_ref::<f32>() {
        return Ok(v.abs() >= EPSILON32);
    }
    if let Some(&v) = value.downcast_ref::<f64>() {
        return Ok(v.abs() >= EPSILON64);
    }
    Err(E3dcError::Type(format!(
        "Cannot convert {:?} to bool",
        (**value).type_id()
    )))
}

/// E3DC client wrapper
pub struct E3dcClient {
    client: Client,
    pub batteries: Vec<BatteryInfo>,
    info: SystemInfoStatic,
}

pub fn empty_item(tag: u32) -> Item {
    Item { tag, data: None }
}

fn find_item<'a>(items: &'a [&'a Item], tag: u32) -> Result<&'a Item, E3dcError> {
    items
        .iter()
        .find(|item| item.tag == tag)
        .copied()
        .ok_or(E3dcError::MissingTag(tag))
}

fn find_item_data<'a>(items: &'a [&'a Item], tag: u32) -> Result<&'a Box<dyn Any>, E3dcError> {
    let item = items
        .iter()
        .find(|item| item.tag == tag)
        .ok_or(E3dcError::MissingTag(tag))?;

    item.data.as_ref().ok_or(E3dcError::MissingData(tag))
}
fn get_items<'a>(items: &'a [&'a Item], tag: u32) -> Result<Vec<&'a Item>, E3dcError> {
    let item = find_item(items, tag)?;
    any_to_items(&item.data)
}

fn get_bool(items: &[&Item], tag: u32) -> Result<bool, E3dcError> {
    let data = find_item_data(items, tag)?;
    any_to_bool(data)
}

fn get_number(items: &[&Item], tag: u32) -> Result<f64, E3dcError> {
    let data = find_item_data(items, tag)?;
    any_to_f64(data)
}

fn get_integer(items: &[&Item], tag: u32) -> Result<u64, E3dcError> {
    let data = find_item_data(items, tag)?;
    any_to_u64(data)
}

fn get_string(items: &[&Item], tag: u32) -> Result<String, E3dcError> {
    let data = find_item_data(items, tag)?;
    any_to_string(data)
}

pub fn send_request(client: &mut Client, frame: Frame) -> Result<Frame, E3dcError> {
    let response = client
        .send_receive_frame(&frame)
        .map_err(|e| E3dcError::QueryFailed(format!("{:?}", e)))?;

    if response.items.is_none() {
        return Err(E3dcError::QueryFailed("Response has no data".to_string()));
    }

    Ok(response)
}

impl E3dcClient {
    /// Create a new E3DC client
    pub fn new(
        host: String,
        key: String,
        username: String,
        password: String,
    ) -> Result<Self, E3dcError> {
        let mut client = Client::new(&key, username, password);
        info!("Connecting to E3DC at {}...", host);
        client
            .connect(&host, None)
            .map_err(|e| E3dcError::ConnectionFailed {
                host: host.clone(),
                reason: format!("{:?}", e),
            })?;
        info!("✓ Connected to E3DC successfully!");
        let batteries = Self::get_batteries(&mut client)?;
        let info = Self::get_system_info_static(&mut client)?;
        let device_id = format!("{}-{}", &info.model, &info.serial_number);
        info!("Device ID: {}", device_id);

        Ok(Self {
            client,
            batteries,
            info,
        })
    }

    pub fn send_request(&mut self, frame: Frame) -> Result<Frame, E3dcError> {
        //Result<(Vec<Item>, DateTime<Utc>), E3dcError> {
        send_request(&mut self.client, frame)
    }

    /// Polls the static system info via rscp protocol.
    pub fn get_system_info_static(client: &mut Client) -> Result<SystemInfoStatic, E3dcError> {
        let mut frame = Frame::new();

        frame.push_item(empty_item(EMS::DERATE_AT_PERCENT_VALUE.into()));
        frame.push_item(empty_item(EMS::DERATE_AT_POWER_VALUE.into()));
        frame.push_item(empty_item(EMS::INSTALLED_PEAK_POWER.into()));
        frame.push_item(empty_item(EMS::EXT_SRC_AVAILABLE.into()));
        frame.push_item(empty_item(INFO::SERIAL_NUMBER.into()));
        frame.push_item(empty_item(INFO::MAC_ADDRESS.into()));

        let result = send_request(client, frame)?;

        let all_items = any_to_items(&result.items)?;

        let derate_at_percent_value = get_number(&all_items, EMS::DERATE_AT_PERCENT_VALUE.into())?;
        let derate_at_power_value = get_integer(&all_items, EMS::DERATE_AT_POWER_VALUE.into())?;
        let installed_peak_power = get_integer(&all_items, EMS::INSTALLED_PEAK_POWER.into())?;
        let ext_source_available = get_bool(&all_items, EMS::EXT_SRC_AVAILABLE.into())?;
        let mac_address: String = get_string(&all_items, INFO::MAC_ADDRESS.into())?;
        let serial: String = get_string(&all_items, INFO::SERIAL_NUMBER.into())?;
        let serial_number = if serial.chars().count() > 4 {
            serial.chars().skip(4).collect()
        } else {
            serial.to_string()
        };
        let model = if serial_number.starts_with("4") || serial_number.starts_with("72") {
            "S10E"
        } else if serial_number.starts_with("74") {
            "S10E_Compact"
        } else if serial_number.starts_with("5") {
            "S10_Mini"
        } else if serial_number.starts_with("6") {
            "Quattroporte"
        } else if serial_number.starts_with("70") {
            "S10E_Pro"
        } else if serial_number.starts_with("75") {
            "S10E_Pro_Compact"
        } else if serial_number.starts_with("8") {
            "S10X"
        } else {
            "N/A"
        };

        Ok(SystemInfoStatic {
            serial_number,
            model,
            mac_address,
            installed_peak_power,
            derate_at_percent_value,
            derate_at_power_value,
            ext_source_available,
        })
    }

    /// Get system information (called once at startup)
    /// Only queries tags that are known to work
    pub fn get_system_info(&'_ mut self) -> Result<SystemInfo<'_>, E3dcError> {
        let mut frame = Frame::new();

        // INFO tags
        frame.push_item(empty_item(INFO::SW_RELEASE.into()));
        frame.push_item(empty_item(INFO::IP_ADDRESS.into()));

        // GET_POWER_SETTINGS returns a container with all power settings
        frame.push_item(empty_item(EMS::GET_POWER_SETTINGS.into()));
        // REQ_GET_SYS_SPECS returns system specifications (battery capacity, AC power, etc.)
        frame.push_item(empty_item(EMS::REQ_GET_SYS_SPECS.into()));

        //let (all_items, time_stamp) = self.send_request(frame)?;

        let response = self.send_request(frame)?;
        let all_items = any_to_items(&response.items)?;
        let time_stamp = response.time_stamp;
        // Extract GET_POWER_SETTINGS container
        let power_settings_items = get_items(&all_items, EMS::GET_POWER_SETTINGS.into())?;

        // Extract SYS_SPECS container (response has same tag as request)
        let sys_specs_items: &Vec<Item> = response.get_item_data(EMS::GET_SYS_SPECS.into())?;

        let specs_map: HashMap<String, u64> = sys_specs_items
            .iter()
            .filter(|item| item.tag == EMS::SYS_SPEC as u32)
            .filter_map(|item| {
                let items = any_to_items(&item.data).ok()?;
                let name = get_string(&items, EMS::SYS_SPEC_NAME.into()).ok()?;
                let value = get_integer(&items, EMS::SYS_SPEC_VALUE_INT.into());
                match value {
                    Ok(v) => Some((name, v)),
                    Err(_) => None,
                }
            })
            .collect();

        let ip_address: String = get_string(&all_items, INFO::IP_ADDRESS.into())?;
        let software_release = get_string(&all_items, INFO::SW_RELEASE.into())?;
        // Extract from SYS_SPECS container
        let installed_battery_capacity = specs_map.get("installedBatteryCapacity").copied();
        let max_ac_power = specs_map.get("maxAcPower").copied();
        let max_battery_charge_power = specs_map.get("maxBatChargePower").copied();
        let max_battery_discharge_power = specs_map.get("maxBatDischargPower").copied();
        // Extract from GET_POWER_SETTINGS container
        let max_charge_power = get_integer(&power_settings_items, EMS::MAX_CHARGE_POWER.into())?;
        let max_discharge_power =
            get_integer(&power_settings_items, EMS::MAX_DISCHARGE_POWER.into())?;
        let discharge_start_power =
            get_integer(&power_settings_items, EMS::DISCHARGE_START_POWER.into())?;
        let power_limits_used = get_bool(&power_settings_items, EMS::POWER_LIMITS_USED.into())?;
        let power_save_enabled = get_bool(&power_settings_items, EMS::POWERSAVE_ENABLED.into())?;
        let weather_forecast_mode =
            get_integer(&power_settings_items, EMS::WEATHER_FORECAST_MODE.into())?;
        let weather_regulated_charge_enabled = get_bool(
            &power_settings_items,
            EMS::WEATHER_REGULATED_CHARGE_ENABLED.into(),
        )?;

        Ok(SystemInfo {
            time_stamp,
            serial_number: &self.info.serial_number,
            model: self.info.model,
            mac_address: &self.info.mac_address,
            ip_address,
            software_release,
            installed_peak_power: self.info.installed_peak_power,
            installed_battery_capacity,
            max_ac_power,
            max_battery_charge_power,
            max_battery_discharge_power,
            derate_percent: self.info.derate_at_percent_value,
            derate_power: self.info.derate_at_power_value,
            max_charge_power,
            max_discharge_power,
            discharge_start_power,
            power_limits_used,
            power_save_enabled,
            weather_forecast_mode,
            weather_regulated_charge_enabled,
            external_source_available: self.info.ext_source_available,
        })
    }

    pub fn batteries(&self) -> &Vec<BatteryInfo> {
        &self.batteries
    }

    /// Get current status (polled every interval)
    /// Queries all status values in one frame
    pub fn get_status(&mut self) -> Result<Status, E3dcError> {
        let mut frame = Frame::new();

        // Request all status values in one frame
        frame.push_item(empty_item(EMS::POWER_PV.into()));
        frame.push_item(empty_item(EMS::POWER_BAT.into()));
        frame.push_item(empty_item(EMS::POWER_GRID.into()));
        frame.push_item(empty_item(EMS::POWER_HOME.into()));
        frame.push_item(empty_item(EMS::BAT_SOC.into()));
        frame.push_item(empty_item(EMS::AUTARKY.into()));
        frame.push_item(empty_item(EMS::SELF_CONSUMPTION.into()));
        frame.push_item(empty_item(EMS::POWER_WB_ALL.into()));
        frame.push_item(empty_item(EMS::POWER_ADD.into()));

        let response = self.send_request(frame)?;

        let time_stamp = response.time_stamp;
        let all_items = any_to_items(&response.items)?;

        // Extract values - use get_item_data for i32 (works reliably)
        let power_add = get_number(&all_items, EMS::POWER_ADD.into())?;
        let power_pv = get_number(&all_items, EMS::POWER_PV.into())?;
        let power_battery = get_number(&all_items, EMS::POWER_BAT.into())?;
        let power_grid = get_number(&all_items, EMS::POWER_GRID.into())?;
        let power_home = get_number(&all_items, EMS::POWER_HOME.into())?;
        let power_wb = get_number(&all_items, EMS::POWER_WB_ALL.into())?;
        let battery_soc = get_number(&all_items, EMS::BAT_SOC.into())?;
        let autarky = get_number(&all_items, EMS::AUTARKY.into())?;
        let self_consumption = get_number(&all_items, EMS::SELF_CONSUMPTION.into())?;

        Ok(Status {
            time_stamp,
            power_add,
            power_pv,
            power_battery,
            power_grid,
            power_home,
            power_wb,
            battery_soc,
            autarky,
            self_consumption,
        })
    }

    /// Scan for installed batteries (up to 8 batteries)
    /// Uses BATCH query - ONE network call instead of 8 (saves ~7 seconds!)
    /// Returns list of BatteryInfo with index and DCB count
    fn get_batteries(client: &mut Client) -> Result<Vec<BatteryInfo>, E3dcError> {
        // Build ONE frame with ALL battery queries (batch optimization)
        let mut frame = Frame::new();
        frame.push_item(empty_item(BAT::REQ_AVAILABLE_BATTERIES.into()));

        // Send ONE request for ALL batteries (saves seconds!)
        let response = client
            .send_receive_frame(&frame)
            .map_err(|e| E3dcError::QueryFailed(format!("Battery batch query failed: {:?}", e)))?;
        let all_items = any_to_items(&response.items)?;
        let available_batteries = get_items(&all_items, BAT::AVAILABLE_BATTERIES.into())?;
        let batteries: Vec<BatteryInfo> = available_batteries
            .into_iter()
            .map(|battery| -> Result<BatteryInfo, E3dcError> {
                let spec = any_to_items(&battery.data)?;
                let index = get_integer(&spec, BAT::INDEX.into())?;
                let param_bat_number = get_integer(&spec, BAT::PARAM_BAT_NUMBER.into())?;
                let device_name = get_string(&spec, BAT::DEVICE_NAME.into())?;
                let manufacturer_name = get_string(&spec, BAT::MANUFACTURER_NAME.into())?;
                let serialno = get_integer(&spec, BAT::SERIALNO.into())?;
                let instance_descriptor = get_string(&spec, BAT::INSTANCE_DESCRIPTOR.into())?;

                let mut frame = Frame::new();
                frame.push_item(Item::new(
                    BAT::DATA.into(),
                    vec![
                        Item {
                            tag: BAT::INDEX.into(),
                            data: Some(Box::new(index as i32)),
                        },
                        Item {
                            tag: BAT::DCB_COUNT.into(),
                            data: None,
                        },
                    ],
                ));
                let response = send_request(client, frame)?;

                let all_items = any_to_items(&response.items)?;
                let data = get_items(&all_items, BAT::DATA.into())?;

                let dcb_count = get_integer(&data, BAT::DCB_COUNT.into())?;

                Ok(BatteryInfo {
                    index,
                    param_bat_number,
                    device_name,
                    manufacturer_name,
                    serialno,
                    instance_descriptor,
                    dcb_count,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(batteries)
    }

    pub fn get_battery_data(&mut self) -> Result<Vec<BatteryData>, E3dcError> {
        let batteries = self.batteries.clone();
        batteries
            .iter()
            .map(|battery| self.get_battery_data_idx(battery))
            .collect()
    }

    /// Get comprehensive battery data for specific battery index
    /// Queries all available battery parameters in one request
    /// Matches Python implementation with all fields
    fn get_battery_data_idx(&mut self, battery: &BatteryInfo) -> Result<BatteryData, E3dcError> {
        let mut frame = Frame::new();

        // Request comprehensive battery data with ALL fields from Python implementation
        frame.push_item(Item::new(
            BAT::DATA.into(),
            vec![
                Item {
                    tag: BAT::INDEX.into(),
                    data: Some(Box::new(battery.index)),
                },
                // State of Charge
                empty_item(BAT::RSOC.into()),
                empty_item(BAT::RSOC_REAL.into()),
                empty_item(BAT::ASOC.into()),
                // Electrical measurements
                empty_item(BAT::CURRENT.into()),
                empty_item(BAT::MODULE_VOLTAGE.into()),
                empty_item(BAT::TERMINAL_VOLTAGE.into()),
                empty_item(BAT::MAX_BAT_VOLTAGE.into()),
                empty_item(BAT::EOD_VOLTAGE.into()),
                // Capacity
                empty_item(BAT::FCC.into()),
                empty_item(BAT::RC.into()),
                empty_item(BAT::DESIGN_CAPACITY.into()),
                empty_item(BAT::USABLE_CAPACITY.into()),
                empty_item(BAT::USABLE_REMAINING_CAPACITY.into()),
                // Current limits
                empty_item(BAT::MAX_CHARGE_CURRENT.into()),
                empty_item(BAT::MAX_DISCHARGE_CURRENT.into()),
                // Temperature
                empty_item(BAT::MAX_DCB_CELL_TEMPERATURE.into()),
                empty_item(BAT::MIN_DCB_CELL_TEMPERATURE.into()),
                // Status and errors
                empty_item(BAT::STATUS_CODE.into()),
                empty_item(BAT::ERROR_CODE.into()),
                // Cycles and usage
                empty_item(BAT::CHARGE_CYCLES.into()),
                empty_item(BAT::TOTAL_USE_TIME.into()),
                empty_item(BAT::TOTAL_DISCHARGE_TIME.into()),
                // DCB info
                empty_item(BAT::DCB_COUNT.into()),
                // Operational state
                empty_item(BAT::READY_FOR_SHUTDOWN.into()),
                empty_item(BAT::TRAINING_MODE.into()),
            ],
        ));

        let response = self.send_request(frame)?;
        let all_items = any_to_items(&response.items)?;

        // Find BAT::DATA container
        let bat_data_items = get_items(&all_items, BAT::DATA.into())?;

        // Build comprehensive battery data response
        Ok(BatteryData {
            index: battery.index,
            time_stamp: response.time_stamp,
            // State of Charge
            rsoc: get_number(&bat_data_items, BAT::RSOC.into())?,
            rsoc_real: get_number(&bat_data_items, BAT::RSOC_REAL.into())?,
            asoc: get_number(&bat_data_items, BAT::ASOC.into())?,
            // Electrical measurements
            current: get_number(&bat_data_items, BAT::CURRENT.into())?,
            module_voltage: get_number(&bat_data_items, BAT::MODULE_VOLTAGE.into())?,
            terminal_voltage: get_number(&bat_data_items, BAT::TERMINAL_VOLTAGE.into())?,
            max_bat_voltage: get_number(&bat_data_items, BAT::MAX_BAT_VOLTAGE.into())?,
            eod_voltage: get_number(&bat_data_items, BAT::EOD_VOLTAGE.into())?,
            // Capacity
            fcc: get_number(&bat_data_items, BAT::FCC.into())?,
            rc: get_number(&bat_data_items, BAT::RC.into())?,
            design_capacity: get_number(&bat_data_items, BAT::DESIGN_CAPACITY.into())?,
            usable_capacity: get_number(&bat_data_items, BAT::USABLE_CAPACITY.into())?,
            usable_remaining_capacity: get_number(
                &bat_data_items,
                BAT::USABLE_REMAINING_CAPACITY.into(),
            )?,
            // Current limits
            max_charge_current: get_number(&bat_data_items, BAT::MAX_CHARGE_CURRENT.into())?,
            max_discharge_current: get_number(&bat_data_items, BAT::MAX_DISCHARGE_CURRENT.into())?,
            // Temperature
            max_dcb_cell_temp: get_number(&bat_data_items, BAT::MAX_DCB_CELL_TEMPERATURE.into())?,
            min_dcb_cell_temp: get_number(&bat_data_items, BAT::MIN_DCB_CELL_TEMPERATURE.into())?,
            // Status and errors
            status_code: get_number(&bat_data_items, BAT::STATUS_CODE.into())?,
            error_code: get_number(&bat_data_items, BAT::ERROR_CODE.into())?,
            // Cycles and usage
            charge_cycles: get_number(&bat_data_items, BAT::CHARGE_CYCLES.into())?,
            total_use_time: get_integer(&bat_data_items, BAT::TOTAL_USE_TIME.into())?,
            total_discharge_time: get_integer(&bat_data_items, BAT::TOTAL_DISCHARGE_TIME.into())?,
            // Device info
            device_name: battery.device_name.clone(),
            // DCB info - use the count from startup, not from the query (which returns 0)
            dcb_count: battery.dcb_count,
            dcbs: (0..battery.dcb_count)
                .map(|idx| self.get_dcb_data(battery.index, idx))
                .collect::<Result<Vec<_>, _>>()?,
            // Operational state
            ready_for_shutdown: get_bool(&bat_data_items, BAT::READY_FOR_SHUTDOWN.into())?,
            training_mode: get_bool(&bat_data_items, BAT::TRAINING_MODE.into())?,
            param_bat_number: battery.param_bat_number,
            instance_descriptor: battery.instance_descriptor.clone(),
            manufacturer_name: battery.manufacturer_name.clone(),
            serialno: battery.serialno,
        })
    }

    /// Extract cell data (temperatures or voltages) from a nested DCB container
    ///
    /// The structure is:
    /// BAT::DCB_ALL_CELL_TEMPERATURES/VOLTAGES (Container)
    ///   ├─ [0] 0x03000100 (unknown field)
    ///   └─ [1] BAT::DATA (Container)
    ///       └─ Multiple BAT::DCB_CELL_TEMPERATURE or BAT::DCB_CELL_VOLTAGE items
    fn extract_dcb_cell_data(container: &[&Item], cell_tag: u32) -> Result<Vec<f64>, E3dcError> {
        get_items(container, BAT::DATA.into())?
            .iter()
            .filter(|item| item.tag == cell_tag)
            .map(|item| {
                let data = item
                    .data
                    .as_ref()
                    .ok_or_else(|| E3dcError::MissingData(item.tag))?;
                any_to_f64(data)
            })
            .collect()
    }

    /// Get DCB (DC Battery Controller) complete information
    /// Queries ALL DCB data including cell voltages and temperatures
    /// Uses the correct Python pye3dc approach with DCB index as value
    ///
    /// Returns complete DcbData with all fields matching Python implementation
    pub fn get_dcb_data(
        &mut self,
        battery_index: u64,
        dcb_index: u64,
    ) -> Result<DcbData, E3dcError> {
        let mut frame = Frame::new();
        frame.push_item(Item::new(
            BAT::DATA.into(),
            vec![
                Item {
                    tag: BAT::INDEX.into(),
                    data: Some(Box::new(battery_index as u16)),
                },
                // Pass DCB index as VALUE to these tags (Python pye3dc method)
                Item {
                    tag: BAT::DCB_ALL_CELL_TEMPERATURES.into(),
                    data: Some(Box::new(dcb_index)),
                },
                Item {
                    tag: BAT::DCB_ALL_CELL_VOLTAGES.into(),
                    data: Some(Box::new(dcb_index)),
                },
                Item {
                    tag: BAT::DCB_INFO.into(),
                    data: Some(Box::new(dcb_index)),
                },
            ],
        ));

        let response = self.send_request(frame)?;
        let all_items = any_to_items(&response.items)?;

        // Find BAT::DATA container
        let container_items = get_items(&all_items, BAT::DATA.into())?;

        // Extract DCB_INFO
        let dcb_info_items = get_items(&container_items, BAT::DCB_INFO.into())?;

        // Get counts
        let sensor_count = get_integer(&dcb_info_items, BAT::DCB_NR_SENSOR.into())?;
        let series_cell_count = get_integer(&dcb_info_items, BAT::DCB_NR_SERIES_CELL.into())?;
        let parallel_cell_count = get_integer(&dcb_info_items, BAT::DCB_NR_PARALLEL_CELL.into())?;

        // Extract temperatures
        let all_temps_vec = get_items(&container_items, BAT::DCB_ALL_CELL_TEMPERATURES.into())?;
        let all_temps =
            Self::extract_dcb_cell_data(&all_temps_vec, BAT::DCB_CELL_TEMPERATURE.into())?;

        let cell_temperatures: Vec<f64> = if sensor_count > 0 {
            all_temps.into_iter().take(sensor_count as usize).collect()
        } else {
            all_temps
                .into_iter()
                .filter(|&temp| temp >= MIN_VALID_CELL_TEMP_C)
                .collect()
        };

        // Extract voltages
        let all_voltages_vec = get_items(&container_items, BAT::DCB_ALL_CELL_VOLTAGES.into())?;
        let all_voltages =
            Self::extract_dcb_cell_data(&all_voltages_vec, BAT::DCB_CELL_VOLTAGE.into())?;

        let cell_voltages: Vec<f64> = if series_cell_count > 0 {
            all_voltages
                .into_iter()
                .take(series_cell_count as usize)
                .collect()
        } else {
            all_voltages
        };

        // If E3DC doesn't provide series_cell_count (returns 0), use actual length
        let actual_series_cell_count = if series_cell_count > 0 {
            series_cell_count
        } else {
            cell_voltages.len() as u64
        };

        Ok(DcbData {
            index: dcb_index,
            // Current measurements
            current: get_number(&dcb_info_items, BAT::DCB_CURRENT.into())?,
            current_avg_30s: get_number(&dcb_info_items, BAT::DCB_CURRENT_AVG_30S.into())?,
            voltage: get_number(&dcb_info_items, BAT::DCB_VOLTAGE.into())?,
            voltage_avg_30s: get_number(&dcb_info_items, BAT::DCB_VOLTAGE_AVG_30S.into())?,
            // State
            soc: get_number(&dcb_info_items, BAT::DCB_SOC.into())?,
            soh: get_number(&dcb_info_items, BAT::DCB_SOH.into())?,
            cycle_count: get_number(&dcb_info_items, BAT::DCB_CYCLE_COUNT.into())?,
            // Capacity
            design_capacity: get_number(&dcb_info_items, BAT::DCB_DESIGN_CAPACITY.into())?,
            design_voltage: get_number(&dcb_info_items, BAT::DCB_DESIGN_VOLTAGE.into())?,
            full_charge_capacity: get_number(
                &dcb_info_items,
                BAT::DCB_FULL_CHARGE_CAPACITY.into(),
            )?,
            remaining_capacity: get_number(&dcb_info_items, BAT::DCB_REMAINING_CAPACITY.into())?,
            // Limits
            max_charge_voltage: get_number(&dcb_info_items, BAT::DCB_MAX_CHARGE_VOLTAGE.into())?,
            max_charge_current: get_number(&dcb_info_items, BAT::DCB_MAX_CHARGE_CURRENT.into())?,
            max_discharge_current: get_number(
                &dcb_info_items,
                BAT::DCB_MAX_DISCHARGE_CURRENT.into(),
            )?,
            end_of_discharge: get_number(&dcb_info_items, BAT::DCB_END_OF_DISCHARGE.into())?,
            max_charge_temperature: get_number(
                &dcb_info_items,
                BAT::DCB_CHARGE_HIGH_TEMPERATURE.into(),
            )?,
            min_charge_temperature: get_number(
                &dcb_info_items,
                BAT::DCB_CHARGE_LOW_TEMPERATURE.into(),
            )?,
            // Device info
            device_name: get_string(&dcb_info_items, BAT::DCB_DEVICE_NAME.into())?,
            manufacture_name: get_string(&dcb_info_items, BAT::DCB_MANUFACTURE_NAME.into())?,
            manufacture_date: get_number(&dcb_info_items, BAT::DCB_MANUFACTURE_DATE.into())?,
            serial_code: get_string(&dcb_info_items, BAT::DCB_SERIALCODE.into())?,
            serial_no: get_number(&dcb_info_items, BAT::DCB_SERIALNO.into())?,
            fw_version: get_number(&dcb_info_items, BAT::DCB_FW_VERSION.into())?,
            pcb_version: get_number(&dcb_info_items, BAT::DCB_PCB_VERSION.into())?,
            protocol_version: get_number(&dcb_info_items, BAT::DCB_PROTOCOL_VERSION.into())?,
            // Status
            error: get_number(&dcb_info_items, BAT::DCB_ERROR.into())?,
            warning: get_number(&dcb_info_items, BAT::DCB_WARNING.into())?,
            status: get_number(&dcb_info_items, BAT::DCB_STATUS.into())?,
            // Cell configuration
            series_cell_count: actual_series_cell_count,
            parallel_cell_count,
            sensor_count,
            // Cell data
            cell_temperatures,
            cell_voltages,
        })
    }

    /// Get daily statistics for today
    pub fn get_daily_statistics(
        &mut self,
        stat_interval: Duration,
    ) -> Result<DailyStatistics, E3dcError> {
        // Get start of today (midnight) in UTC
        let now = Utc::now();

        let timespan = Duration::seconds(now.num_seconds_from_midnight().into());
        let start = now - timespan;

        if timespan <= stat_interval {
            let yesterday = start - Duration::days(1);
            self.get_db_data_timestamp(yesterday, Duration::hours(1))
        } else {
            self.get_db_data_timestamp(start, timespan)
        }
    }

    /// Get database statistics for a specific timespan
    pub fn get_db_data_timestamp(
        &mut self,
        start: DateTime<Utc>,
        timespan: Duration,
    ) -> Result<DailyStatistics, E3dcError> {
        let mut frame = Frame::new();

        // Create DB_REQ_HISTORY_DATA_DAY container with time parameters
        let time_params = vec![
            Item {
                tag: DB::HISTORY_TIME_START.into(),
                data: Some(Box::new(u64::try_from(start.timestamp()).map_err(
                    |_| E3dcError::ParseError(format!("Invalid timestamp: {}", start)),
                )?)),
            },
            Item {
                tag: DB::HISTORY_TIME_INTERVAL.into(),
                data: Some(Box::new(timespan.num_seconds())),
            },
            Item {
                tag: DB::HISTORY_TIME_SPAN.into(),
                data: Some(Box::new(timespan.num_seconds())),
            },
        ];

        frame.push_item(Item {
            tag: DB::HISTORY_DATA_DAY.into(),
            data: Some(Box::new(time_params)),
        });

        let response = self.client.send_receive_frame(&frame)?;

        let time_stamp = response.time_stamp;
        let all_items = any_to_items(&response.items)?;
        // Extract SUM_CONTAINER from response

        // Find HISTORY_DATA_DAY response container
        let history_container = get_items(&all_items, DB::HISTORY_DATA_DAY.into())?;

        // Find SUM_CONTAINER within history data
        let sum_container = get_items(&history_container, DB::SUM_CONTAINER.into())?;

        // Helper to extract values from SUM_CONTAINER

        Ok(DailyStatistics {
            time_stamp,
            autarky: get_number(&sum_container, DB::AUTARKY.into())?,
            consumed_production: get_number(&sum_container, DB::CONSUMED_PRODUCTION.into())?,
            solar_production: get_number(&sum_container, DB::DC_POWER.into())?,
            consumption: get_number(&sum_container, DB::CONSUMPTION.into())?,
            bat_power_in: get_number(&sum_container, DB::BAT_POWER_IN.into())?,
            bat_power_out: get_number(&sum_container, DB::BAT_POWER_OUT.into())?,
            grid_power_in: get_number(&sum_container, DB::GRID_POWER_IN.into())?,
            grid_power_out: get_number(&sum_container, DB::GRID_POWER_OUT.into())?,
            state_of_charge: get_number(&sum_container, DB::BAT_CHARGE_LEVEL.into())?,
            start,
            timespan,
        })
    }
}

impl Drop for E3dcClient {
    fn drop(&mut self) {
        tracing::info!("Disconnecting E3DC client...");
        if let Err(e) = self.client.disconnect() {
            tracing::warn!("Error disconnecting E3DC: {:?}", e);
        } else {
            tracing::info!("E3DC client disconnected");
        }
    }
}
