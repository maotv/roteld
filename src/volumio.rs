

// #[macro_use]
// extern crate serde_derive;

// extern crate serde;
extern crate serde_json;

extern crate ws;

use ws::{Handler, Handshake, Result, Message};
use ws::Error as WsError;

use serde_json::Value;

use std::sync::mpsc;
// use std::sync::mpsc::{Sender,Receiver,TryRecvError};

use common::Event;

#[derive(Serialize, Deserialize)]
#[serde(default = "VolumioState::default")]
struct VolumioState {

    status: String,
    position: i64,
    title: String,
    artist: String,
    album: String,
    albumart: String,
    trackType: String,
    seek: i64,
    duration: i64,
    samplerate: String,
    bitdepth: String,
    channels: i64,
    random: Value,
    repeat: Value,
    repeatSingle: bool,
    consume: bool,
    volume: i64,
    mute: bool,
    stream: String,
    updatedb: bool,
    volatile: bool,
    service: String,

}

impl VolumioState {

    fn default() -> VolumioState {
        VolumioState {
            status: String::from(""),
            position: 0,
            title: String::from(""),
            artist: String::from(""),
            album: String::from(""),
            albumart: String::from(""),
            trackType: String::from(""),
            seek: 0,
            duration: 0,
            samplerate: String::from(""),
            bitdepth: String::from(""),
            channels: 0,
            random: Value::Bool(false),
            repeat: Value::Bool(false),
            repeatSingle: false,
            consume: false,
            volume: 0,
            mute: false,
            stream: String::from(""),
            updatedb: false,
            volatile: false,
            service: String::from(""),
        }
    }


}



// Our Handler struct.
// Here we explicity indicate that the Client needs a Sender,
// whereas a closure captures the Sender for us automatically.
pub struct WsToVolumio {
    pub out: ws::Sender,
    pub tx:  mpsc::Sender<Event>,
}


// We implement the Handler trait for Client so that we can get more
// fine-grained control of the connection.
impl Handler for WsToVolumio {

    // `on_open` will be called only after the WebSocket handshake is successful
    // so at this point we know that the connection is ready to send/receive messages.
    // We ignore the `Handshake` for now, but you could also use this method to setup
    // Handler state or reject the connection based on the details of the Request
    // or Response, such as by checking cookies or Auth headers.
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        // Now we don't need to call unwrap since `on_open` returns a `Result<()>`.
        // If this call fails, it will only result in this connection disconnecting.
        println!("Volumio Open, send probe...");
        self.out.send("2probe");
        self.tx.send(Event::WsConnect( self.out.clone() ));
        Ok(())
    }

    fn on_error(&mut self, err: WsError) {
        println!("<<< Error<{:?}>", err);
    }

    // `on_message` is roughly equivalent to the Handler closure. It takes a `Message`
    // and returns a `Result<()>`.
    fn on_message(&mut self, msg: Message) -> Result<()> {
        // Close the connection when we get a response from the server
        // println!("[Volumio] {}", msg);
        let jstr: String = msg.into_text().unwrap();
        if jstr.starts_with("42[") {

            let mut state: Value = serde_json::from_str(&jstr[2..]).unwrap();

            if state[0] == "pushState" {
                println!("Got 42: {}", jstr);
                let vstate: Value = state.as_array_mut().unwrap().remove(1);
                self.tx.send(Event::Volumio(vstate));

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



