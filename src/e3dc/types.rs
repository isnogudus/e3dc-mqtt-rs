//! Data types for E3DC responses
//!
//! These types mirror the data structures from the Python implementation

use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone)]
pub struct SystemInfoStatic {
    pub serial_number: String,
    pub model: &'static str,
    pub mac_address: String,
    pub installed_peak_power: u64,
    pub derate_at_percent_value: f64,
    pub derate_at_power_value: u64,
    pub ext_source_available: bool,
}
/// System information (retrieved once at startup)
#[derive(Debug, Clone)]
pub struct SystemInfo<'a> {
    pub time_stamp: DateTime<Utc>,
    pub serial_number: &'a String,
    pub mac_address: &'a String,
    pub ip_address: String,
    pub model: &'static str,
    pub software_release: String,
    pub installed_peak_power: u64,                // W
    pub installed_battery_capacity: Option<u64>,  // Wh
    pub max_ac_power: Option<u64>,                // W
    pub max_battery_charge_power: Option<u64>,    // W
    pub max_battery_discharge_power: Option<u64>, // W
    // Power management
    pub derate_percent: f64,        // % (Derate at percent value)
    pub derate_power: u64,          // W (Derate at power value)
    pub max_charge_power: u64,      // W (EMS max charge power)
    pub max_discharge_power: u64,   // W (EMS max discharge power)
    pub discharge_start_power: u64, // W
    pub power_limits_used: bool,
    pub power_save_enabled: bool,
    // Weather regulation
    pub weather_forecast_mode: u64,
    pub weather_regulated_charge_enabled: bool,
    // External source (not available in rscp tags, set to 0)
    pub external_source_available: bool,
}

/// Current status (polled every interval, e.g., 5s)
#[derive(Debug, Clone)]
pub struct Status {
    pub time_stamp: DateTime<Utc>,
    pub power_battery: f64, // W (positive = charging, negative = discharging)
    pub power_wb: f64,      // W (positive = charging, negative = discharging)
    pub power_home: f64,    // W
    pub power_pv: f64,      // W
    pub power_grid: f64,    // W (positive = to grid, negative = from grid)
    pub power_add: f64,     // W
    pub battery_soc: f64,   // %
    pub autarky: f64,       // %
    pub self_consumption: f64, // %
}
/// Battery data (polled at longer interval, e.g., 300s)
/// Comprehensive battery information matching Python implementation
#[derive(Debug, Clone, PartialEq)]
pub struct BatteryData {
    pub index: u64,
    pub time_stamp: DateTime<Utc>,

    // State of Charge
    pub rsoc: f64,      // Relative State of Charge %
    pub rsoc_real: f64, // Real Relative State of Charge %
    pub asoc: f64,      // Absolute State of Charge %

    // Electrical measurements
    pub current: f64,          // A
    pub module_voltage: f64,   // V
    pub terminal_voltage: f64, // V
    pub max_bat_voltage: f64,  // V
    pub eod_voltage: f64,      // End of Discharge voltage (V)

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
    pub param_bat_number: u64,
    pub manufacturer_name: String,
    pub serialno: u64,
    pub instance_descriptor: String,

    // DCB info
    pub dcb_count: u64,
    pub dcbs: Vec<DcbData>, // DCB modules (DC Battery Controllers) with detailed cell data

    // Operational state
    pub ready_for_shutdown: bool,
    pub training_mode: bool,
}

/// Daily statistics (polled at longer interval)
#[derive(Debug, Clone)]
pub struct DailyStatistics {
    pub time_stamp: DateTime<Utc>,
    pub autarky: f64,             // %
    pub consumption: f64,         // %
    pub solar_production: f64,    // Wh
    pub consumed_production: f64, // Wh
    pub bat_power_in: f64,        // Wh
    pub bat_power_out: f64,       // Wh
    pub grid_power_in: f64,       // Wh
    pub grid_power_out: f64,      // Wh
    pub state_of_charge: f64,     // %
    pub start: DateTime<Utc>,     // Unix timestamp
    pub timespan: Duration,       // Duration in seconds
}

/// Battery info (index and DCB count)
#[derive(Debug, Clone)]
pub struct BatteryInfo {
    pub index: u64,
    pub device_name: String,
    pub param_bat_number: u64,
    pub manufacturer_name: String,
    pub serialno: u64,
    pub instance_descriptor: String,
    pub dcb_count: u64,
}

/// DCB (DC Battery Controller) detailed information
/// Matches Python pye3dc implementation
#[derive(Debug, Clone, PartialEq)]
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
    pub series_cell_count: u64, // Returns u32, often 0 (not provided by E3DC)
    pub parallel_cell_count: u64, // Returns u32, often 0 (not provided by E3DC)
    pub sensor_count: u64,
    // Cell data
    pub cell_temperatures: Vec<f64>, // °C (from BAT::DCB_ALL_CELL_TEMPERATURES)
    pub cell_voltages: Vec<f64>,     // V (from BAT::DCB_ALL_CELL_VOLTAGES)
}
