
extern crate ws;

// use ws::{Handler, Handshake, Result, Message};
use serde_json::Value;

pub struct KeyValue {
    pub name:  String,
    pub value: String
}



pub enum Event {

    Rotel(KeyValue),
    Volumio(Value),
    Serial(String),
    WsConnect(ws::Sender),
    WsPing

}

