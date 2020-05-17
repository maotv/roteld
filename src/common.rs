
extern crate ws;

// use ws::{Handler, Handshake, Result, Message};
use serde_json::Value;

#[derive(Debug)]
pub struct KeyValueRaw {
    pub key:  String,
    pub value: String,
    pub raw: String
}



pub enum Event {

    // RotelMessage(KeyValueRaw),
    RotelNormVolume(f64),
    VolumioState(Value),
    SerialData(String),
    SocketSerialBroadcaster(ws::Sender),
    VolumioConnect(ws::Sender),
    VolumioPing

}


pub fn normal_volume(min: i64, max: i64, value: i64) -> f64 {
    ((value - min) as f64 / (max - min) as f64).max(0.0).min(1.0)
}

pub fn device_volume(min: i64, max: i64, value: f64) -> i64 {
    (((value  * (max-min) as f64) + min as f64 ) as i64).max(min).min(max)
}



