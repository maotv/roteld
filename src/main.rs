#[macro_use]
extern crate serde_derive;

extern crate argparse;
extern crate serialport;
extern crate ws;
extern crate serde_json;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod common;
mod rwc; // rotel-web-client
mod volumio;
mod rotel;

use volumio::Volumio;
use common::Event;
use rotel::Rotel;
use rotel::RotelCommand;
use rwc::SocketSerial;

use std::fs::File;
use std::io::Read;

const ROTEL_VOLUME_MIN: i64 = 1;
const ROTEL_VOLUME_MAX: i64 = 64;

const VOLUMIO_VOLUME_MIN: i64 = 0;
const VOLUMIO_VOLUME_MAX: i64 = 100;

fn normal_volume(min: i64, max: i64, value: i64) -> f64 {
    ((value - min) as f64 / (max - min) as f64).max(0.0).min(1.0)
}

fn device_volume(min: i64, max: i64, value: f64) ->i64 {
    ((value  * (max-min) as f64) + min as f64 ) as i64
}


#[derive(Deserialize)]
struct Setup {
    volumio_url: String,
    rotel_serial: String
}

fn main() {

    let args: Vec<String> = std::env::args().collect();
    println!("{:?}", args);

    let setup_default = String::from("setup.json");
    let setup_json = args.get(0).unwrap_or(&setup_default);


    let mut file = File::open(setup_json).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();

    let setup: Setup = serde_json::from_str(&data).expect("cannot read setup json");


    let (tx_event, rx_event) = mpsc::channel();
    let (tx_command, rx_command) = mpsc::channel();

    let mut amp: Rotel = Rotel::new(&setup.rotel_serial);

    let txc = tx_event.clone();
    amp.start(txc, rx_command);


        let txc = tx_event.clone();
        thread::spawn(move || {
            Volumio::connect(&setup.volumio_url /*"ws://127.0.0.1:3000/socket.io/?EIO=3&transport=websocket"*/, txc);
        });

        let txc = tx_event.clone();
        thread::spawn(move || {
            SocketSerial::listen("0.0.0.0:8989", txc);
        });


        let txc = tx_event.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(23000));
                check(txc.send(Event::VolumioPing));
            }
        });



        let mut volumio_current_volume: i64 = 0;

        let mut volumio_sender: Option<ws::Sender> = None;

        let mut rwc_out: Option<ws::Sender> = None;


        // ===================================================================
        //
        //     Main Event Loop
        //
        // ===================================================================
        loop {

            println!("[Loop   ] --------------- Main Event Loop -------------");
            // amp.check();

            match rx_event.recv() {

                Ok(Event::RotelMessage(ur)) => {
                    // println!("[Main   ] Rotel Event: {}", ur.name );

                    rwc_out = rwc_out.map( |out| {
                        println!("[Rotel] pass serial message to rwc");
                        if ur.name == "display" {
                            check(out.send(format!("{} \"D\": \"{}\" {}","{", &ur.raw[..20], "}")));
                            check(out.send(format!("{} \"D\": \"{}\" {}","{", &ur.raw[20..], "}")));
                        } else {
                            check(out.send(format!("{} \"D\": \"{}\" {}","{", ur.raw, "}")));
                        }
                        out
                    });


                    if ur.name == "volume" {

                        let rotvol = rotel::parse_rotel_volume(&ur.value);
                        // println!("[Main   ] Rotel Event: Volume {}", rotvol);
                        check(tx_command.send(RotelCommand::Received(rotvol)));

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
                                check(out.send(format!("429[\"volume\", {}]", volumio_current_volume)));
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
                            check(tx_command.send(RotelCommand::Target(rotel_target_volume)));
                        }


                    } else {
                        println!("[Main   ] Volumio Event, Volume unchanged ({})", ps_volume);
                    }

                }, 

                Ok(Event::SerialData(msg)) => {
                    println!("[Main   ] Serial Event ({})", msg);
                    check(tx_command.send(RotelCommand::Command(msg)));
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
                        check(out.send("2"));
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

fn check<T>(res: Result<(), T>) 
    where T: std::fmt::Debug
{
    match res {
        Ok(()) => (),
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}








