
extern crate ws;

// use ws::{Handler, Handshake, Result, Message};
use serde_json::Value;

pub struct KeyValueRaw {
    pub name:  String,
    pub value: String,
    pub raw: String
}



pub enum Event {

    RotelMessage(KeyValueRaw),
    VolumioState(Value),
    SerialData(String),
    SocketSerialBroadcaster(ws::Sender),
    VolumioConnect(ws::Sender),
    VolumioPing

}

