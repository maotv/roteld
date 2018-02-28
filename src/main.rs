extern crate argparse;
extern crate serialport;
extern crate ws;
extern crate serde_json;

use ws::WebSocket;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use serialport::prelude::*;
use serialport::posix::TTYPort;
use std::os::unix::prelude::*;
use std::path::Path;

mod common;
mod rwc; // rotel-web-client
mod volumio;
mod rotel;


use volumio::Volumio;

use common::Event;
// use common::KeyValue;

use rotel::Rotel;
use rotel::RotelCommand;

use rwc::SocketSerial;

// const EMPTY: &'static str = "";

const ROTEL_VOLUME_MIN: i64 = 1;
const ROTEL_VOLUME_MAX: i64 = 64;

const VOLUMIO_VOLUME_MIN: i64 = 0;
const VOLUMIO_VOLUME_MAX: i64 = 100;


// struct VolumioState {
//     volume: i64,
// }






// impl VolumioState {

//     fn new() -> VolumioState {
//         VolumioState {
//             status: String::from(""),
//             position: 0,
//             title: String::from(""),
//             artist: String::from(""),
//             album: String::from(""),
//             albumart: String::from(""),
//             trackType: String::from(""),
//             seek: 0,
//             duration: 0,
//             samplerate: String::from(""),
//             bitdepth: String::from(""),
//             channels: 0,
//             random: false,
//             repeat: false,
//             repeatSingle: false,
//             consume: false,
//             volume: 0,
//             mute: false,
//             stream: String::from(""),
//             updatedb: false,
//             volatile: false,
//             service: String::from(""),
//         }
//     }

// }







fn normal_volume(min: i64, max: i64, value: i64) -> f64 {
    ((value - min) as f64 / (max - min) as f64).max(0.0).min(1.0)
}

fn device_volume(min: i64, max: i64, value: f64) ->i64 {
    ((value  * (max-min) as f64) + min as f64 ) as i64
}





fn main() {


    // let port_name = "/dev/ttyUSB0";

    // let settings = SerialPortSettings {

    //     baud_rate: BaudRate::Baud115200,
    //     data_bits: DataBits::Eight,
    //     flow_control: FlowControl::None,
    //     parity: Parity::None,
    //     stop_bits: StopBits::One,
    //     timeout: Duration::from_millis(1),

    // };


//    let (tx_v, rx_v) = mpsc::channel();
    let (tx_event, rx_event) = mpsc::channel();
    // let tx_event_vol  = tx_event.clone();
    // let tx_event_r    = tx_event.clone();
    // let tx_event_ping = tx_event.clone();
    // let tx_event_dummy = tx_event.clone();
    //let tx2 = mpsc::Sender::clone(&tx1);

    let (tx_command, rx_command) = mpsc::channel();


    // let (tx_command_dummy, rx_command_dummy) = mpsc::channel();



    let amp: Rotel = Rotel::new();

    let txc = tx_event.clone();
    amp.start(txc, rx_command);


//    if let Ok(mut port) = TTYPort::open(Path::new(port_name), &settings) {

        // let fd_read  = port.as_raw_fd();
        // // let fd_write = port.as_raw_fd(); // clone??

        // println!("port is open! #{}", fd_read);

        // thread::spawn(move || {
        //     rotel::rotel_reader_thread(fd_read, tx_event_r);
        // });

        // thread::spawn(move || {
        //     rotel::rotel_command_thread(fd_read, rx_command);
        // });



        // thread::spawn(move || {

        //     loop {

        //         println!("[Setup  ] Client Connect to Volumio Websocket");
        //         ws::connect("ws://127.0.0.1:3000/socket.io/?EIO=3&transport=websocket", |out| volumio::Volumio { out: out, tx: tx_event_vol.clone() } ).unwrap();
        //         println!("[Setup  ] Client Connection closed");
        //         thread::sleep(Duration::from_millis(300));
        //     }

        // });

//        let wstb = WebSocket::new( |out| rwc::SocketSerial { out: out, tx: tx_event_rwc.clone()  } ).unwrap();
        // let wssender = wstb.broadcaster().clone();

        let txc = tx_event.clone();
        thread::spawn(move || {
            Volumio::connect("ws://127.0.0.1:3000/socket.io/?EIO=3&transport=websocket", txc);
        });



        let txc = tx_event.clone();
        thread::spawn(move || {
            SocketSerial::listen("192.168.178.53:8989", txc);
        });


        let txc = tx_event.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(23000));
                txc.send(Event::VolumioPing);
            }
        });


        // thread::spawn(move || {

        //     let wstb = WebSocket::new( |out| rwc::SocketSerial { out: out, tx: tx_event_rwc.clone()  } ).unwrap();
        //     tx_event_rwc.send(Event::RwcBroadcaster(wstb.broadcaster().clone()));

        //     //    tx_event_rwc.clone();
        //     //loop {
        //         wstb.listen( "192.168.178.53:8989" ).unwrap();
        //         thread::sleep(Duration::from_millis(300));
        //     // }

        // });





       // let t_alsa = thread::spawn(move || {
       //      main_alsa_thread(fd_write, rx_r, rx_v);
       //  });

        // A WebSocket echo server
        // let rotelWebClient = rwc::SocketSerial { out: None };
        // ws::listen("127.0.0.1:8989", |out| rotelWebClient.with_out(out) ).unwrap();



//        let wstb = new WebSocket( |out| rwc::SocketSerial { out: out, tx: tx_rwc } );




        let mut volumio_current_volume: i64 = 0;
        // let mut rotel_current_volume: i64 = 0;
        // let mut rotel_target_volume: i64  = 0;

        let mut volumio_sender: Option<ws::Sender> = None;

        let mut rwc_out: Option<ws::Sender> = None;


        // ===================================================================
        //
        //     Main Event Loop
        //
        // ===================================================================
        loop {

            println!("[Loop   ] --------------- Main Event Loop -------------");

            match rx_event.recv() {

                Ok(Event::RotelMessage(ur)) => {
                    // println!("[Main   ] Rotel Event: {}", ur.name );

                    rwc_out = rwc_out.map( |out| {
                        println!("[Rotel] pass serial message to rwc");
                        if ur.name == "display" {
                            out.send(format!("{} \"D\": \"{}\" {}","{", &ur.raw[..20], "}"));
                            out.send(format!("{} \"D\": \"{}\" {}","{", &ur.raw[20..], "}"));
                        } else {
                            out.send(format!("{} \"D\": \"{}\" {}","{", ur.raw, "}"));
                        }
                        out
                    });


                    if ur.name == "volume" {

                        let rotvol = rotel::parse_rotel_volume(&ur.value);
                        // println!("[Main   ] Rotel Event: Volume {}", rotvol);
                        tx_command.send(RotelCommand::Received(rotvol));

                        if !rotel::rotel_is_adjusting() {
                            rotel::rotel_knob_set_timestamp();
                        }

                        if rotel::rotel_knob_is_turning() {
                            println!("[Main   ] (set.) Rotel => Volumio {}", rotvol);
                            //rotel_target_volume  = rvr;
                            //rotel_current_volume = rvr;
                            let vnorm = normal_volume(ROTEL_VOLUME_MIN, ROTEL_VOLUME_MAX, rotvol);
                            volumio_current_volume = device_volume(VOLUMIO_VOLUME_MIN, VOLUMIO_VOLUME_MAX, vnorm);
                            volumio_sender = volumio_sender.map( |out| {
                                out.send(format!("429[\"volume\", {}]", volumio_current_volume));
                                out
                            });
                        } else {
                            // ??? rotel_current_volume = rotvol;
                            println!("[Main   ] (ign.) Rotel => Volumio {}", rotvol);
                        }
                    } else {
                        println!("[Main   ] Rotel Event: Other {} = {}", ur.name, ur.value);
                    }

                }, 

                Ok(Event::VolumioState(ps)) => {
                    
                    let ps_volume: i64 = ps["volume"].as_i64().unwrap();

                    if ps_volume != volumio_current_volume {

                        volumio_current_volume = ps_volume;

                        if rotel::rotel_knob_is_turning() { // n second timeout when directly setting rotel volume
                            println!("[Main   ] (ign.) Volumio Event, Volume is {} (was: {})", ps_volume, volumio_current_volume);
                        } else {
                            println!("[Main   ] (set.) Volumio Event, Volume is {} (was: {})", ps_volume, volumio_current_volume);
                            let vnorm = normal_volume(VOLUMIO_VOLUME_MIN, VOLUMIO_VOLUME_MAX, volumio_current_volume);
                            let rotel_target_volume = device_volume(ROTEL_VOLUME_MIN, ROTEL_VOLUME_MAX, vnorm);
                            tx_command.send(RotelCommand::Target(rotel_target_volume));
                        }


                    } else {
                        println!("[Main   ] Volumio Event, Volume unchanged ({})", ps_volume);
                    }

                }, 

                Ok(Event::SerialData(msg)) => {
                    println!("[Main   ] Serial Event ({})", msg);
                    tx_command.send(RotelCommand::Command(msg));
                },

                Ok(Event::SocketSerialBroadcaster(snd)) => {
                    println!("[Main   ] Got Broadcaster");
                    rwc_out = Some(snd);
                },



                Ok(Event::VolumioConnect(snd)) => {

                    volumio_sender = Some(snd);
                },

                Ok(Event::VolumioPing) => {
                    volumio_sender = volumio_sender.map( |out| {
                        println!("[Volumio] send ping");
                        out.send("2");
                        out
                    });
                },

                Err(..) => {
                    println!("Something wrong");
                }

            }



            

        }


        
    //}



}










