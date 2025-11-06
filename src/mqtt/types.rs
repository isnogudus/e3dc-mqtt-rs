use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use crate::e3dc;

fn round(value: f64, decimals: i32) -> f64 {
    let multiplier = 10_f64.powi(decimals);
    (value * multiplier).round() / multiplier
}

pub struct Status {
    pub time: DateTime<Utc>,
    pub additional: f64,
    pub autarky: f64,
    pub battery_charge: f64,
    pub battery_discharge: f64,
    pub battery_consumption: f64,
    pub consumption_from_grid: f64,
    pub export_to_grid: f64,
    pub grid_production: f64,
    pub house_consumption: f64,
    pub self_consumption: f64,
    pub solar_production: f64,
    pub solar_production_excess: f64,
    pub state_of_charge: f64,
    pub wb_consumption: f64,
}

/// Splits a signed value into positive and negative parts.
///
/// Returns `(value, 0)` if positive, `(0, abs(value))` if negative.
/// Used for separating bidirectional metrics (e.g., grid import/export).
fn split_val(value: f64) -> (f64, f64) {
    if value >= 0.0 {
        (value, 0.0)
    } else {
        (0.0, value.abs())
    }
}

impl Status {
    pub fn from_e3dc(status: &e3dc::Status) -> Self {
        let additional = -status.power_add;
        // Split power_battery into charge/discharge (Python compatibility)
        let (battery_charge, battery_discharge) = split_val(status.power_battery);

        // Split power_grid into from_grid/to_grid (Python compatibility)
        // power_grid > 0: consuming from grid, power_grid < 0: exporting to grid
        let (consumption_from_grid, export_to_grid) = split_val(status.power_grid);

        let solar_production_excess = status.power_pv - status.power_home;

        Status {
            time: status.time_stamp,
            additional,
            autarky: round(status.autarky, 1),
            battery_charge,
            battery_discharge,
            battery_consumption: status.power_battery,
            consumption_from_grid,
            export_to_grid,
            grid_production: status.power_grid,
            house_consumption: status.power_home,
            self_consumption: round(status.self_consumption, 1),
            solar_production: status.power_pv,
            solar_production_excess,
            state_of_charge: status.battery_soc,
            wb_consumption: status.power_wb,
        }
    }
}

#[derive(Serialize)]
pub struct SystemInfo<'a> {
    pub time: DateTime<Utc>,
    pub derate_percent: f64, // % (Derate at percent value)
    pub derate_power: u64,   // W (Derate at power value)
    // External source (not available in rscp tags, set to 0)
    pub external_source_available: bool,
    pub installed_battery_capacity: Option<u64>, // Wh
    pub installed_peak_power: u64,               // W
    pub ip_address: &'a String,
    pub max_ac_power: Option<u64>, // W
    pub mac_address: &'a String,
    pub max_battery_charge_power: Option<u64>,    // W
    pub max_battery_discharge_power: Option<u64>, // W
    pub model: &'static str,
    pub release: &'a String,
    pub serial: &'a String,

    // Power management
    pub discharge_start_power: u64, // W
    pub max_charge_power: u64,      // W (EMS max charge power)
    pub max_discharge_power: u64,   // W (EMS max discharge power)
    pub power_limits_used: bool,
    pub power_save_enabled: bool,
    // Weather regulation
    pub weather_forecast_mode: u64,
    pub weather_regulated_charge_enabled: bool,
}

impl<'a> SystemInfo<'a> {
    pub fn from_e3dc(info: &'a e3dc::SystemInfo) -> Self {
        let derate_percent = round(info.derate_percent, 2);
        Self {
            time: info.time_stamp,
            derate_percent,
            derate_power: info.derate_power,
            external_source_available: info.external_source_available,
            installed_battery_capacity: info.installed_battery_capacity,
            installed_peak_power: info.installed_peak_power,
            ip_address: &info.ip_address,
            max_ac_power: info.max_ac_power,
            mac_address: info.mac_address,
            max_battery_charge_power: info.max_battery_charge_power,
            max_battery_discharge_power: info.max_battery_discharge_power,
            model: info.model,
            release: &info.software_release,
            serial: info.serial_number,
            discharge_start_power: info.discharge_start_power,
            max_charge_power: info.max_charge_power,
            max_discharge_power: info.max_discharge_power,
            power_limits_used: info.power_limits_used,
            power_save_enabled: info.power_save_enabled,
            weather_forecast_mode: info.weather_forecast_mode,
            weather_regulated_charge_enabled: info.weather_regulated_charge_enabled,
        }
    }
}

pub struct DcbData {
    pub index: u64,
    // Current measurements
    pub current: f64,         // A
    pub current_avg_30s: f64, // A
    pub voltage: f64,         // V
    pub voltage_avg_30s: f64, // V
    // State
    pub soc: f64, // %
    pub soh: f64, // % State of Health
    pub cycle_count: f64,
    // Capacity
    pub design_capacity: f64,      // Ah
    pub design_voltage: f64,       // V
    pub full_charge_capacity: f64, // Ah
    pub remaining_capacity: f64,   // Ah
    // Limits
    pub max_charge_voltage: f64,     // V
    pub max_charge_current: f64,     // A
    pub max_discharge_current: f64,  // A
    pub end_of_discharge: f64,       // V
    pub max_charge_temperature: f64, // °C
    pub min_charge_temperature: f64, // °C
    // Device info
    pub device_name: String,
    pub manufacture_name: String,
    pub manufacture_date: f64, // Unix timestamp
    pub serial_code: String,
    pub serial_no: f64, // Serial number as integer
    pub fw_version: f64,
    pub pcb_version: f64,
    pub protocol_version: f64,
    // Status
    pub error: f64,
    pub warning: f64,
    pub status: f64,
    // Cell configuration
    // Note: E3DC often returns 0 for these values (firmware limitation).
    // Use cell_voltages.len() for actual series cell count.
    pub series_cell_count: u64, // Returns u64, often 0 (not provided by E3DC)
    pub parallel_cell_count: u64, // Returns u64, often 0 (not provided by E3DC)
    pub sensor_count: u64,
    // Cell data
    pub temperatures: Vec<f64>, // °C (from BAT::DCB_ALL_CELL_TEMPERATURES)
    pub voltages: Vec<f64>,     // V (from BAT::DCB_ALL_CELL_VOLTAGES)
}

impl DcbData {
    pub fn from_e3dc(data: &e3dc::DcbData) -> Self {
        Self {
            index: data.index,
            current: round(data.current, 2),
            current_avg_30s: round(data.current_avg_30s, 2),
            cycle_count: data.cycle_count,
            design_capacity: data.design_capacity,
            design_voltage: round(data.design_voltage, 2),
            device_name: data.device_name.clone(),
            end_of_discharge: data.end_of_discharge,
            error: data.error,
            full_charge_capacity: data.full_charge_capacity,
            fw_version: data.fw_version,
            manufacture_date: data.manufacture_date,
            manufacture_name: data.manufacture_name.clone(),
            max_charge_current: data.max_charge_current,
            max_charge_temperature: data.max_charge_temperature,
            max_charge_voltage: round(data.max_charge_voltage, 2),
            max_discharge_current: data.max_discharge_current,
            min_charge_temperature: data.min_charge_temperature,
            parallel_cell_count: data.parallel_cell_count,
            sensor_count: data.sensor_count,
            series_cell_count: data.series_cell_count,
            pcb_version: data.pcb_version,
            protocol_version: data.protocol_version,
            remaining_capacity: round(data.remaining_capacity, 2),
            serial_no: data.serial_no,
            serial_code: data.serial_code.clone(),
            soc: round(data.soc, 2),
            soh: data.soh,
            status: data.status,
            temperatures: data
                .cell_temperatures
                .iter()
                .map(|t| round(*t, 2))
                .collect(),
            voltage: round(data.voltage, 2),
            voltage_avg_30s: round(data.voltage_avg_30s, 2),
            voltages: data.cell_voltages.iter().map(|v| round(*v, 2)).collect(),
            warning: data.warning,
        }
    }
}

pub struct BatteryData {
    pub index: u64,
    pub time: DateTime<Utc>,

    // State of Charge
    pub rsoc: f64,      // Relative State of Charge %
    pub rsoc_real: f64, // Real Relative State of Charge %
    pub asoc: f64,      // Absolute State of Charge %

    // Electrical measurements
    pub current: f64,             // A
    pub module_voltage: f64,      // V
    pub terminal_voltage: f64,    // V
    pub max_battery_voltage: f64, // V
    pub eod_voltage: f64,         // End of Discharge voltage (V)

    // Capacity
    pub fcc: f64,                       // Full Charge Capacity (Ah)
    pub rc: f64,                        // Remaining Capacity (Ah)
    pub design_capacity: f64,           // Design Capacity (Ah)
    pub usable_capacity: f64,           // Usable Capacity (Ah)
    pub usable_remaining_capacity: f64, // Usable Remaining Capacity (Ah)

    // Current limits
    pub max_charge_current: f64,    // A
    pub max_discharge_current: f64, // A

    // Temperature
    pub max_dcb_cell_temp: f64, // °C
    pub min_dcb_cell_temp: f64, // °C

    // Status and errors
    pub status_code: f64,
    pub error_code: f64,

    // Cycles and usage
    pub charge_cycles: f64,
    pub total_use_time: u64,       // seconds
    pub total_discharge_time: u64, // seconds

    // Device info
    pub device_name: String,

    // DCB info
    pub dcb_count: u64,
    pub dcbs: Vec<DcbData>, // DCB modules (DC Battery Controllers) with detailed cell data

    // Operational state
    pub ready_for_shutdown: bool,
    pub training_mode: bool,
}
impl BatteryData {
    pub fn from_e3dc(data: &e3dc::BatteryData) -> Self {
        Self {
            time: data.time_stamp,
            asoc: data.asoc,
            charge_cycles: data.charge_cycles,
            current: round(data.current, 2),
            dcb_count: data.dcb_count,
            dcbs: data.dcbs.iter().map(DcbData::from_e3dc).collect(),
            design_capacity: data.design_capacity,
            device_name: data.device_name.clone(),
            eod_voltage: data.eod_voltage,
            error_code: data.error_code,
            fcc: data.fcc,
            index: data.index,
            max_battery_voltage: round(data.max_bat_voltage, 2),
            max_charge_current: data.max_charge_current,
            max_discharge_current: data.max_discharge_current,
            max_dcb_cell_temp: round(data.max_dcb_cell_temp, 2),
            min_dcb_cell_temp: round(data.min_dcb_cell_temp, 2),
            module_voltage: round(data.module_voltage, 2),
            rc: round(data.rc, 2),
            ready_for_shutdown: data.ready_for_shutdown,
            rsoc: round(data.rsoc, 2),
            rsoc_real: round(data.rsoc_real, 2),
            status_code: data.status_code,
            terminal_voltage: round(data.terminal_voltage, 2),
            total_use_time: data.total_use_time,
            total_discharge_time: data.total_discharge_time,
            training_mode: data.training_mode,
            usable_capacity: round(data.usable_capacity, 2),
            usable_remaining_capacity: round(data.usable_remaining_capacity, 2),
        }
    }
}

pub struct DailyStatistics {
    pub time: DateTime<Utc>,
    pub autarky_today: f64,               // %
    pub self_consumption_today: f64,      // %
    pub solar_production_today: f64,      // Wh
    pub house_consumption_today: f64,     // Wh
    pub battery_charge_today: f64,        // Wh
    pub battery_discharge_today: f64,     // Wh
    pub export_to_grid_today: f64,        // Wh
    pub consumption_from_grid_today: f64, // Wh
    pub state_of_charge_today: f64,       // %
    pub start: DateTime<Utc>,             // Unix timestamp
    pub timespan: Duration,               // Duration in seconds
}

impl DailyStatistics {
    pub fn from_e3dc(stat: &e3dc::DailyStatistics) -> Self {
        Self {
            time: stat.time_stamp,
            autarky_today: round(stat.autarky, 1),
            battery_charge_today: stat.bat_power_in,
            battery_discharge_today: stat.bat_power_out,
            self_consumption_today: round(stat.consumed_production, 1),
            house_consumption_today: stat.consumption,
            export_to_grid_today: stat.grid_power_in,
            consumption_from_grid_today: stat.grid_power_out,
            start: stat.start,
            state_of_charge_today: round(stat.state_of_charge, 1),
            solar_production_today: stat.solar_production,
            timespan: stat.timespan,
        }
    }
}
