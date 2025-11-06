# E3DC RSCP Tags Reference

Dokumentation der wichtigsten RSCP Tags für die E3DC-MQTT Bridge.

## Verfügbare Tag-Gruppen

Aus `rscp::tags::*`:

- **INFO** - Systeminformationen (Serial, MAC, SW-Release, Model)
- **EMS** - Energy Management System (Power-Werte, Autarkie, etc.)
- **BAT** - Batterie-Daten (SOC, Voltage, Current, DCBs)
- **DB** - Datenbank / Statistiken (Tageswerte, Historien)
- **PM** - Power Meter (Externe Zähler)
- **PVI** - PV Inverter (Wechselrichter-Daten)
- **SYS** - System-Befehle und Status

## Mapping Python → Rust Tags

### System Info (einmalig beim Start)

| Python-Methode | RSCP Tag | Typ | Beschreibung |
|---------------|----------|-----|--------------|
| `get_system_info()` | - | - | Kombination mehrerer Tags |
| → `serial` | `INFO::SERIAL_NUMBER` | String | Seriennummer |
| → `macAddress` | `INFO::MAC_ADDRESS` | String | MAC-Adresse |
| → `model` | `INFO::A35_SERIAL_NUMBER` oder ähnlich | String | Modell-Bezeichnung |
| → `release` | `INFO::SW_RELEASE` | String | Software-Version |
| → `installedPeakPower` | `EMS::INSTALLED_PEAK_POWER` | i32 | Installierte PV-Leistung (W) |
| → `installedBatteryCapacity` | `EMS::INSTALLED_BATTERY_CAPACITY` | i32 | Installierte Batterie-Kapazität (Wh) |
| → `maxAcPower` | `EMS::MAX_POWER` | i32 | Max. AC-Leistung (W) |
| → `maxBatChargePower` | `EMS::MAX_CHARGE_POWER` | i32 | Max. Lade-Leistung (W) |
| → `maxBatDischargePower` | `EMS::MAX_DISCHARGE_POWER` | i32 | Max. Entlade-Leistung (W) |

### Status (alle 5s)

| Python-Methode | RSCP Tag | Typ | Beschreibung |
|---------------|----------|-----|--------------|
| `poll()` | - | - | Kombination mehrerer Tags |
| → `production.solar` | `EMS::POWER_PV` | i32 | PV-Produktion (W) |
| → `production.grid` | `EMS::POWER_GRID` | i32 | Netz (W, + = Einspeisung, - = Bezug) |
| → `consumption.battery` | `EMS::POWER_BAT` | i32 | Batterie (W, + = Ladung, - = Entladung) |
| → `consumption.house` | `EMS::POWER_HOME` | i32 | Hausverbrauch (W) |
| → `stateOfCharge` | `EMS::BAT_SOC` oder `BAT::RSOC` | f32 | Batterieladezustand (%) |
| → `autarky` | `EMS::AUTARKY` | f32 | Autarkie (%) |
| → `selfConsumption` | `EMS::SELF_CONSUMPTION` | f32 | Eigenverbrauch (%) |

### Batterie-Details (alle 300s)

| Python-Methode | RSCP Tag | Typ | Beschreibung |
|---------------|----------|-----|--------------|
| `get_batteries_data()` | `BAT::DATA` (Container) | - | Batterie-Informationen |
| → `index` | `BAT::INDEX` | u8 | Batterie-Index |
| → `rsoc` | `BAT::RSOC` | f32 | Relativer SOC (%) |
| → `asoc` | `BAT::ASOC` | f32 | Absoluter SOC (%) |
| → `current` | `BAT::CURRENT` | f32 | Strom (A) |
| → `moduleVoltage` | `BAT::MODULE_VOLTAGE` | f32 | Spannung (V) |
| → `terminalVoltage` | `BAT::TERMINAL_VOLTAGE` | f32 | Klemmenspannung (V) |
| → `statusCode` | `BAT::STATUS_CODE` | u32 | Status-Code |
| → `errorCode` | `BAT::ERROR_CODE` | u32 | Fehler-Code |
| → `chargeCycles` | `BAT::CHARGE_CYCLES` | u32 | Ladezyklen |
| → `fcc` | `BAT::FCC` | f32 | Full Charge Capacity (Ah) |
| → `rc` | `BAT::RC` | f32 | Remaining Capacity (Ah) |

### DCB (Battery Control Boards)

| Python-Methode | RSCP Tag | Beschreibung |
|---------------|----------|--------------|
| → `dcbs[].soc` | `BAT::DCB_INFO` Container | DCB State of Charge (%) |
| → `dcbs[].voltage` | `BAT::DCB_INFO` | DCB Spannung (V) |
| → `dcbs[].current` | `BAT::DCB_INFO` | DCB Strom (A) |
| → `dcbs[].temperatures` | `BAT::DCB_INFO` | DCB Temperaturen (°C) |

### Statistiken / Tageswerte (alle 300s)

| Python-Methode | RSCP Tag | Typ | Beschreibung |
|---------------|----------|-----|--------------|
| `get_db_data_timestamp()` | `DB::REQ_HISTORY_DATA_DAY` | - | Tages-Statistiken |
| → `autarky` | - | f32 | Tages-Autarkie (%) |
| → `solarProduction` | - | u32 | Tages-Solar-Ertrag (Wh) |
| → `consumption` | - | u32 | Tages-Verbrauch (Wh) |
| → `bat_power_in` | - | u32 | Tages-Batterieladung (Wh) |
| → `bat_power_out` | - | u32 | Tages-Batterieentladung (Wh) |
| → `grid_power_in` | - | u32 | Tages-Netzeinspeisung (Wh) |
| → `grid_power_out` | - | u32 | Tages-Netzbezug (Wh) |

## Hinweise

1. **Container-Tags**: Manche Tags wie `BAT::DATA` sind Container, die mehrere Sub-Items enthalten
2. **Signed vs Unsigned**: Achten auf korrekte Typen (i32 für signed Power-Werte)
3. **Units**:
   - Leistung: Watt (W) als i32/u32
   - Energie: Wattstunden (Wh) als u32
   - Spannung: Volt (V) als f32
   - Strom: Ampere (A) als f32
   - Prozent: % als f32

## TODO: Zu klärende Tags

- Exakte Tag-Namen für DB/Statistiken (Python nutzt `pye3dc`, das evtl. eigene Wrapper hat)
- Model-Bezeichnung (evtl. über `INFO::PRODUCTION_DATE` kombiniert?)
- Power Settings (`get_power_settings()` im Python-Code)
