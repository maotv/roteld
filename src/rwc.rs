
extern crate ws;
extern crate serde_json;


use ws::{Handler, Handshake, Result, Message};
// use ws::Error as WsError;


use serde_json::Value;
use common::Event;
use std::sync::mpsc;


pub struct WsToBrowser {
    pub out: ws::Sender,
    pub tx: mpsc::Sender<Event>
}



// We implement the Handler trait for Client so that we can get more
// fine-grained control of the connection.
impl Handler for WsToBrowser {

    // `on_open` will be called only after the WebSocket handshake is successful
    // so at this point we know that the connection is ready to send/receive messages.
    // We ignore the `Handshake` for now, but you could also use this method to setup
    // Handler state or reject the connection based on the details of the Request
    // or Response, such as by checking cookies or Auth headers.
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        // Now we don't need to call unwrap since `on_open` returns a `Result<()>`.
        // If this call fails, it will only result in this connection disconnecting.
        println!("On Open...");
       // self.out.send("2probe")
       Ok(())
    }

    // `on_message` is roughly equivalent to the Handler closure. It takes a `Message`
    // and returns a `Result<()>`.
    fn on_message(&mut self, msg: Message) -> Result<()> {
        // Close the connection when we get a response from the server
        println!("Got message: {}", msg);
        

        let jstr: String = msg.into_text().unwrap();
        if jstr.starts_with("sendjson ") {

            let rmsg: Value = serde_json::from_str(&jstr[9..]).unwrap();
            let cmd = rmsg["Data"][0]["D"].as_str().unwrap();
            if cmd.starts_with("get_current_power")  {
                self.out.send("{ \"D\": \"power=on!\"}").unwrap();
            } 
            println!("Json message: {}", cmd);
            self.tx.send(Event::Serial(String::from(cmd)));
            println!("Sent message: {}", cmd);
            // send to rotel device
        }

        
        Ok(())
        // self.out.close(CloseCode::Normal)
    }
}



