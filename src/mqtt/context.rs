use chrono::{DateTime, Duration, Utc};
use rumqttc::{Client, QoS};

use crate::errors::MqttError;

pub trait MqttPayload {
    fn to_payload(&self) -> String;
}

impl MqttPayload for DateTime<Utc> {
    fn to_payload(&self) -> String {
        self.to_rfc3339()
    }
}

impl MqttPayload for Duration {
    fn to_payload(&self) -> String {
        self.to_string()
    }
}

impl MqttPayload for Vec<f64> {
    fn to_payload(&self) -> String {
        format!(
            "[{}]",
            self.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

impl MqttPayload for String {
    fn to_payload(&self) -> String {
        self.clone()
    }
}

impl MqttPayload for bool {
    fn to_payload(&self) -> String {
        self.to_string()
    }
}

impl MqttPayload for f64 {
    fn to_payload(&self) -> String {
        self.to_string()
    }
}

impl MqttPayload for u64 {
    fn to_payload(&self) -> String {
        self.to_string()
    }
}

pub struct PublishContext<'a> {
    client: &'a Client,
    pub topic: String,
    pub qos: QoS,
    pub retain: bool,
}

impl<'a> PublishContext<'a> {
    pub fn new(client: &'a Client, topic: impl Into<String>) -> Self {
        Self {
            client,
            topic: topic.into(),
            qos: QoS::AtLeastOnce,
            retain: true,
        }
    }
    pub fn publish<T: MqttPayload>(&self, topic: &str, payload: &T) -> Result<(), MqttError> {
        let full_topic = format!("{}/{}", self.topic, topic);
        self.client
            .publish(&full_topic, self.qos, self.retain, payload.to_payload())
            .map_err(|e| MqttError::PublishFailed {
                topic: full_topic,
                reason: e.to_string(),
            })
    }
}
