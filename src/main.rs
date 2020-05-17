#[macro_use]
extern crate serde_derive;

extern crate argparse;
extern crate serialport;
extern crate ws;
extern crate serde_json;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use log::{info,debug,warn,error};

mod common;
// mod rwc; // rotel-web-client
mod volumio;
mod rotel;

use volumio::Volumio;
use common::Event;
use rotel::RotelDevice;
use rotel::RotelEvent;


use std::fs::File;
use std::io::Read;

use env_logger::Env;


#[derive(Deserialize)]
struct Setup {
    volumio_url: String,
    rotel_serial: String
}

fn main() {

    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", "roteld=trace")
        .write_style_or("MY_LOG_STYLE", "always");

    env_logger::init_from_env(env);

    let args: Vec<String> = std::env::args().collect();
    info!("{:?}", args);

    let setup_default = String::from("setup.json");
    let setup_json = args.get(1).unwrap_or(&setup_default);


    let mut file = File::open(setup_json).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();

    let setup: Setup = serde_json::from_str(&data).expect("cannot read setup json");


    let mut amp = RotelDevice::new(&setup.rotel_serial);


    let (tx_event, rx_event) = mpsc::channel();
    // let (tx_rotel, rx_rotel) = mpsc::channel();

    let to_rotel = amp.start(tx_event.clone());

    let mut volumio: Option<Volumio> = None;

    let mut volumio_current_volume = 0;

    thread::spawn(move || {
        Volumio::connect(&setup.volumio_url, tx_event);
    });




    loop {

        println!("[Loop   ] --------------- Main Event Loop -------------");
        // amp.check();

        match rx_event.recv() {

            // Ok(Event::VolumioState(ps)) => {
                    
            //     let ps_volume: i64 = ps["volume"].as_i64().unwrap();

            //     if ps_volume != volumio_current_volume {

            //         volumio_current_volume = ps_volume;
            //         let vnorm = common::normal_volume(VOLUMIO_VOLUME_MIN, VOLUMIO_VOLUME_MAX, volumio_current_volume);
            //         check(to_rotel.send(RotelEvent::VolumeTarget(vnorm)));

            //     } else {
            //         println!("[Main   ] Volumio Event, Volume unchanged ({})", ps_volume);
            //     }

            // }, 

            Ok(Event::VolumioNormVolume(v)) => {

            },

            Ok(Event::RotelNormVolume(v)) => {

                volumio = volumio.map( |out| {
                    out.send_norm_volume(v);
                    out
                });
                
                // if let Some(vxol) = volumio {
                //     vxol.send_norm_volume(v)
                // }

                // volumio_current_volume = common::device_volume(VOLUMIO_VOLUME_MIN, VOLUMIO_VOLUME_MAX, v);
                // volumio_sender = volumio_sender.map( |out| {
                //     check(out.send(format!("429[\"volume\", {}]", volumio_current_volume)));
                //     out
                // });

            },

            Ok(Event::VolumioConnect(snd)) => {

                volumio = Some(snd);
            },

            Ok(Event::VolumioPing) => {
                volumio = volumio.map( |out| {
                    println!("[Volumio] send ping");
                    out.send_pong();
                    out
                });
            },

            Err(..) => {
                println!("Something wrong");
            }

            _ => {
                println!("case not covered");
            }

        }
    }


}

//     let (tx_event, rx_event) = mpsc::channel();
   


//     let txc = tx_event.clone();
//     let to_rotel = amp.start(txc, rx_command);


//         let txc = tx_event.clone();

// //        thread::spawn(move || {
// //            Volumio::connect(&setup.volumio_url /*"ws://127.0.0.1:3000/socket.io/?EIO=3&transport=websocket"*/, txc);
// //        });

//         let txc = tx_event.clone();
// //        thread::spawn(move || {
// //            SocketSerial::listen("0.0.0.0:8989", txc);
// //        });


//         let txc = tx_event.clone();
// //        thread::spawn(move || {
// //            loop {
// //                thread::sleep(Duration::from_millis(23000));
// //                check(txc.send(Event::VolumioPing));
// //            }
// //        });



//         let mut volumio_current_volume: i64 = 0;

//         let mut volumio_sender: Option<ws::Sender> = None;

//         let mut rwc_out: Option<ws::Sender> = None;


//         // ===================================================================
//         //
//         //     Main Event Loop
//         //
//         // ===================================================================
//         loop {

//             println!("[Loop   ] --------------- Main Event Loop -------------");
//             // amp.check();

//             match rx_event.recv() {

//                 Ok(Event::RotelMessage(ur)) => {
//                     // println!("[Main   ] Rotel Event: {}", ur.name );

//                     rwc_out = rwc_out.map( |out| {
//                         println!("[Rotel] pass serial message to rwc");
//                         if ur.key == "display" {
//                             check(out.send(format!("{} \"D\": \"{}\" {}","{", &ur.raw[..20], "}")));
//                             check(out.send(format!("{} \"D\": \"{}\" {}","{", &ur.raw[20..], "}")));
//                         } else {
//                             check(out.send(format!("{} \"D\": \"{}\" {}","{", ur.raw, "}")));
//                         }
//                         out
//                     });


//                     if ur.key == "volume" {

//                     } else {
//                         println!("[Main   ] Rotel Event: Other {} = {}", ur.key, ur.value);
//                     }

//                 }, 

//                 Ok(Event::RotelNormVolume(v)) => {

//                     volumio_current_volume = common::device_volume(VOLUMIO_VOLUME_MIN, VOLUMIO_VOLUME_MAX, v);
//                     volumio_sender = volumio_sender.map( |out| {
//                         check(out.send(format!("429[\"volume\", {}]", volumio_current_volume)));
//                         out
//                     });

//                 },


//                 Ok(Event::VolumioState(ps)) => {
                    
//                     let ps_volume: i64 = ps["volume"].as_i64().unwrap();

//                     if ps_volume != volumio_current_volume {

//                         volumio_current_volume = ps_volume;

//                         if rotel::rotel_knob_is_turning() { // n second timeout when directly setting rotel volume
//                             println!("[Main   ] (ign.) Volumio Event, Volume is {} (was: {})", ps_volume, volumio_current_volume);
//                         } else {
//                             println!("[Main   ] (set.) Volumio Event, Volume is {} (was: {})", ps_volume, volumio_current_volume);
//                             let vnorm = common::normal_volume(VOLUMIO_VOLUME_MIN, VOLUMIO_VOLUME_MAX, volumio_current_volume);
//                             check(tx_command.send(RotelEvent::VolumeTarget(vnorm)));
//                         }


//                     } else {
//                         println!("[Main   ] Volumio Event, Volume unchanged ({})", ps_volume);
//                     }

//                 }, 

//                 Ok(Event::SerialData(msg)) => {
//                     println!("[Main   ] Serial Event ({})", msg);
//                     check(tx_command.send(RotelEvent::Command(msg)));
//                 },

//                 Ok(Event::SocketSerialBroadcaster(snd)) => {
//                     println!("[Main   ] Got Broadcaster");
//                     rwc_out = Some(snd);
//                 },



//                 Ok(Event::VolumioConnect(snd)) => {

//                     volumio_sender = Some(snd);
//                 },

//                 Ok(Event::VolumioPing) => {
//                     volumio_sender = volumio_sender.map( |out| {
//                         println!("[Volumio] send ping");
//                         check(out.send("2"));
//                         out
//                     });
//                 },

//                 Err(..) => {
//                     println!("Something wrong");
//                 }

//             }



            

//         }


        
//     //}



// }

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








