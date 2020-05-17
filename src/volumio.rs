extern crate serde_json;

extern crate ws;

use ws::{Handler, Handshake, Result, Message};
use ws::Error as WsError;

use log::{trace,info,warn,error};


use serde_json::Value;

use std::sync::mpsc;

use crate::common::Event;

use std::thread;
use std::time::Duration;

use crate::common::*;

const VOLUMIO_VOLUME_MIN: i64 = 0;
const VOLUMIO_VOLUME_MAX: i64 = 100;


#[derive(Clone)]
pub struct Volumio {
    sender: ws::Sender
}


impl Volumio {

    pub fn connect(url: &str, tx: mpsc::Sender<Event>) {

 
        loop {

            println!("[Setup  ] Client Connect to Volumio Websocket");
            ws::connect( url /*"ws://192.168.178.53:3000/socket.io/?EIO=3&transport=websocket"*/, 
                |out| VolumioCore::new(out,tx.clone())).unwrap();
//            ws::connect("ws://127.0.0.1:3000/socket.io/?EIO=3&transport=websocket", |out| Volumio { out: out, tx: tx.clone() } ).unwrap();
            println!("[Setup  ] Client Connection closed");
            thread::sleep(Duration::from_millis(3000));
            
        }

    }

    pub fn send_norm_volume(&self, vol: f64) {

        let dvol = device_volume(VOLUMIO_VOLUME_MIN, VOLUMIO_VOLUME_MAX, vol);
        info!("send volume to volumio {}", dvol);
        self.sender.send(format!("429[\"volume\", {}]", dvol));
    }

    pub fn send_ping(&self) {
        self.sender.send("2");
    }

}


// Our Handler struct.
// Here we explicity indicate that the Client needs a Sender,
// whereas a closure captures the Sender for us automatically.
pub struct VolumioCore {
    out: ws::Sender,
    to_main:  mpsc::Sender<Event>,

    current_volume: i64,
}

impl VolumioCore {

    pub fn new(out: ws::Sender, tx: mpsc::Sender<Event>) -> Self {
        VolumioCore {
            out: out,
            to_main: tx,
            current_volume: 0
        }
    }







    fn handle_push_state(&mut self, state: Value) {

        let ps_volume: i64 = state["volume"].as_i64().unwrap();

        if ps_volume != self.current_volume {

            self.current_volume = ps_volume;
            let vnorm = crate::common::normal_volume(VOLUMIO_VOLUME_MIN, VOLUMIO_VOLUME_MAX, ps_volume);
            self.to_main.send(Event::VolumioNormVolume(vnorm));
//            check(to_rotel.send(RotelEvent::VolumeTarget(vnorm)));

        } else {
            println!("[Main   ] Volumio Event, Volume unchanged ({})", ps_volume);
        }


    }


    // pub fn sender(self) -> Option<ws::Sender> {
    //     Some(self.out.clone())
    // }


}




// We implement the Handler trait for Client so that we can get more
// fine-grained control of the connection.
impl Handler for VolumioCore {

    // `on_open` will be called only after the WebSocket handshake is successful
    // so at this point we know that the connection is ready to send/receive messages.
    // We ignore the `Handshake` for now, but you could also use this method to setup
    // Handler state or reject the connection based on the details of the Request
    // or Response, such as by checking cookies or Auth headers.
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        // Now we don't need to call unwrap since `on_open` returns a `Result<()>`.
        // If this call fails, it will only result in this connection disconnecting.
        println!("Volumio Open, send probe...");
        self.out.send("2probe").unwrap();
        self.to_main.send(Event::VolumioConnect( Volumio { sender: self.out.clone() } )).unwrap();
        Ok(())
    }

    fn on_error(&mut self, err: WsError) {
        println!("<<< Error<{:?}>", err);
    }

    // `on_message` is roughly equivalent to the Handler closure. It takes a `Message`
    // and returns a `Result<()>`.
    fn on_message(&mut self, msg: Message) -> Result<()> {
        // Close the connection when we get a response from the server
        println!("[Volumio] {}", msg);
        let jstr: String = msg.into_text().unwrap();
        if jstr.starts_with("42[") {

            let mut state: Value = serde_json::from_str(&jstr[2..]).unwrap();

            if state[0] == "pushState" {
                // println!("Got 42: {}", jstr);
                let vstate: Value = state.as_array_mut().unwrap().remove(1);
                self.handle_push_state(vstate);
//                self.tx.send(Event::VolumioState(vstate)).expect("cannot send volumio state");

//                let vstate: VolumioState = serde_json::from_value(state.as_array_mut().unwrap().remove(1)).unwrap();
//                self.tx.send(Event::Volumio(vstate));
            } 

        } 
        // else if jstr == "3" {
        //     println!("[Volumio] got pong");
        // }
        
        Ok(())

    }


}



