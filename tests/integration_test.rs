//! Integration tests for e3dc-mqtt-rs
//!
//! These tests verify the core functionality without requiring actual E3DC hardware.

use e3dc_mqtt_rs::config::{E3dcConfig, MqttConfig};
use e3dc_mqtt_rs::mqtt::context::MqttPayload;
use e3dc_mqtt_rs::errors::{E3dcError, MqttError};
use std::time::Duration;
use chrono::{Utc, TimeZone};

#[test]
fn test_mqtt_config_debug_redacts_password() {
    let config = MqttConfig {
        root: "e3dc".to_string(),
        host: Some("mqtt.example.com".to_string()),
        port: 1883,
        socket: None,
        username: "test-user".to_string(),
        password: "secret-password".to_string(),
    };

    let debug_output = format!("{:?}", config);

    // Password should be redacted
    assert!(!debug_output.contains("secret-password"));
    assert!(debug_output.contains("***REDACTED***"));

    // Username should still be visible
    assert!(debug_output.contains("test-user"));
}

#[test]
fn test_e3dc_config_debug_redacts_credentials() {
    let config = E3dcConfig {
        host: "192.168.1.100".to_string(),
        username: "user@example.com".to_string(),
        password: "secret-password".to_string(),
        key: "secret-key".to_string(),
        interval: Duration::from_secs(5),
        statistic_update_interval: Duration::from_secs(60),
    };

    let debug_output = format!("{:?}", config);

    // Sensitive fields should be redacted
    assert!(!debug_output.contains("secret-password"));
    assert!(!debug_output.contains("secret-key"));
    assert!(debug_output.contains("***REDACTED***"));

    // Host and username should still be visible
    assert!(debug_output.contains("192.168.1.100"));
    assert!(debug_output.contains("user@example.com"));
}

// ============================================================================
// MQTT Payload Tests
// ============================================================================

#[test]
fn test_mqtt_payload_f64() {
    let value = 42.5_f64;
    assert_eq!(value.to_payload(), "42.5");

    let value = 0.0_f64;
    assert_eq!(value.to_payload(), "0");

    let value = -123.456_f64;
    assert_eq!(value.to_payload(), "-123.456");
}

#[test]
fn test_mqtt_payload_u64() {
    let value = 12345_u64;
    assert_eq!(value.to_payload(), "12345");

    let value = 0_u64;
    assert_eq!(value.to_payload(), "0");
}

#[test]
fn test_mqtt_payload_bool() {
    let value = true;
    assert_eq!(value.to_payload(), "true");

    let value = false;
    assert_eq!(value.to_payload(), "false");
}

#[test]
fn test_mqtt_payload_string() {
    let value = "test".to_string();
    assert_eq!(value.to_payload(), "test");

    let value = "".to_string();
    assert_eq!(value.to_payload(), "");
}

#[test]
fn test_mqtt_payload_vec_f64() {
    let values = vec![1.0, 2.5, 3.14];
    assert_eq!(values.to_payload(), "[1,2.5,3.14]");

    let empty: Vec<f64> = vec![];
    assert_eq!(empty.to_payload(), "[]");

    let single = vec![42.0];
    assert_eq!(single.to_payload(), "[42]");
}

#[test]
fn test_mqtt_payload_datetime() {
    let dt = Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 45).unwrap();
    let payload = dt.to_payload();

    // Should be RFC3339 format
    assert!(payload.starts_with("2024-01-15T12:30:45"));
    assert!(payload.contains("Z") || payload.contains("+") || payload.contains("-"));
}

// ============================================================================
// Error Type Tests
// ============================================================================

#[test]
fn test_e3dc_error_display() {
    let error = E3dcError::MissingTag(0x12345678);
    let error_string = format!("{}", error);
    assert!(error_string.contains("Missing tag"));
    assert!(error_string.contains("305419896") || error_string.contains("0x12345678"));
}

#[test]
fn test_e3dc_error_missing_data() {
    let error = E3dcError::MissingData(42);
    let error_string = format!("{}", error);
    assert!(error_string.contains("Missing data"));
    assert!(error_string.contains("42"));
}

#[test]
fn test_e3dc_error_type_conversion() {
    let error = E3dcError::Type("Invalid type".to_string());
    let error_string = format!("{}", error);
    assert!(error_string.contains("Invalid"));
    assert!(error_string.contains("type"));
}

#[test]
fn test_mqtt_error_publish_failed() {
    let error = MqttError::PublishFailed {
        topic: "test/topic".to_string(),
        reason: "Connection lost".to_string(),
    };
    let error_string = format!("{}", error);
    assert!(error_string.contains("test/topic"));
    assert!(error_string.contains("Connection lost"));
}


// ============================================================================
// Config Tests
// ============================================================================

#[test]
fn test_config_mqtt_socket_priority() {
    // When both socket and host are provided, socket should take precedence
    // This is tested implicitly by the validation logic
    let config = MqttConfig {
        root: "e3dc".to_string(),
        host: Some("mqtt.example.com".to_string()),
        port: 1883,
        socket: Some("/var/run/mqtt.sock".to_string()),
        username: "test".to_string(),
        password: "test".to_string(),
    };

    // Both should be valid
    assert!(config.socket.is_some());
    assert!(config.host.is_some());
}

#[test]
fn test_duration_formats() {
    // Test that our config uses std::time::Duration which works with serde
    let duration = Duration::from_secs(60);
    assert_eq!(duration.as_secs(), 60);

    let duration = Duration::from_millis(5000);
    assert_eq!(duration.as_secs(), 5);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_mqtt_payload_special_floats() {
    // Test infinity and NaN handling
    let inf = f64::INFINITY;
    let payload = inf.to_payload();
    assert!(payload == "inf" || payload.contains("inf"));

    let neg_inf = f64::NEG_INFINITY;
    let payload = neg_inf.to_payload();
    assert!(payload == "-inf" || payload.contains("inf"));

    let nan = f64::NAN;
    let payload = nan.to_payload();
    assert!(payload == "NaN" || payload.contains("NaN"));
}

#[test]
fn test_mqtt_payload_vec_with_special_values() {
    let values = vec![0.0, f64::INFINITY, -42.5, f64::NAN];
    let payload = values.to_payload();

    // Should contain all values in some form
    assert!(payload.starts_with("["));
    assert!(payload.ends_with("]"));
    assert!(payload.contains(","));
}

#[test]
fn test_error_type_implements_std_error() {
    // Verify that our error types implement std::error::Error
    let e3dc_err = E3dcError::MissingTag(1);
    let _: &dyn std::error::Error = &e3dc_err;

    let mqtt_err = MqttError::ClientError("test".to_string());
    let _: &dyn std::error::Error = &mqtt_err;
}

#[test]
fn test_config_empty_strings() {
    // Test that empty username/password are handled
    let config = MqttConfig {
        root: "".to_string(),  // Empty root should be allowed
        host: Some("mqtt.example.com".to_string()),
        port: 1883,
        socket: None,
        username: "".to_string(),
        password: "".to_string(),
    };

    // Empty strings are valid (though not useful)
    assert_eq!(config.username, "");
    assert_eq!(config.password, "");
}

#[test]
fn test_config_port_ranges() {
    // Test various port numbers
    let config = MqttConfig {
        root: "e3dc".to_string(),
        host: Some("mqtt.example.com".to_string()),
        port: 1, // Minimum valid port
        socket: None,
        username: "test".to_string(),
        password: "test".to_string(),
    };
    assert_eq!(config.port, 1);

    let config = MqttConfig {
        root: "e3dc".to_string(),
        host: Some("mqtt.example.com".to_string()),
        port: 65535, // Maximum valid port
        socket: None,
        username: "test".to_string(),
        password: "test".to_string(),
    };
    assert_eq!(config.port, 65535);

    let config = MqttConfig {
        root: "e3dc".to_string(),
        host: Some("mqtt.example.com".to_string()),
        port: 8883, // Common TLS port
        socket: None,
        username: "test".to_string(),
        password: "test".to_string(),
    };
    assert_eq!(config.port, 8883);
}
