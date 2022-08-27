#[macro_use]
extern crate serde_derive;

extern crate argparse;
extern crate serialport;
extern crate ws;
extern crate serde_json;

use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use log::{info,debug,warn,error};

mod common;
// mod rwc; // rotel-web-client
// mod twinkly;
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
    let to_rotel = amp.start(tx_event.clone());

 
    let mut volumio = Volumio::new(&setup.volumio_url, tx_event.clone());
    let to_volumio = volumio.connect();


    let mut volumio_current_volume = 0;

    // let tx_event_clone = tx_event.clone();
    // thread::spawn(move || {
    //     Volumio::connect(&setup.volumio_url, tx_event_clone);
    // });

    // twinkly::Twinkly::new("").start();

    let tx_clone = tx_event.clone();
    thread::spawn(move || {
        loop {
            if let Err(e) = udp_receiver(tx_clone.clone()) {
                warn!("UDP Thread exited with {:?}", e)
            }
            thread::sleep(Duration::from_millis(1000));
        }
    });



    loop {

        println!("[Loop   ] --------------- Main Event Loop -------------");
        // amp.check();

        match rx_event.recv() {

            Ok(Event::UdpTargetVolume(v)) => {
                to_rotel.send(RotelEvent::VolumeTarget((v as f64) / 100.0));
            },

            Ok(Event::VolumioNormVolume(v)) => {
                to_rotel.send(RotelEvent::VolumeTarget(v));
            },

            Ok(Event::RotelNormVolume(v)) => {
                volumio.send_norm_volume(v)
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


pub fn test_serde() {
    let j = serde_json::to_string(&NetworkMessage::Volume(42)).unwrap();
    info!("{}", j);
}


#[derive(Serialize,Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMessage {
    Volume(usize),
    Power(bool),
    Source(usize),
    Error(String)
    // volume: Option<usize>,
    // power: Option<bool>,
    // source: Option<usize>,
}

fn udp_receiver(tx: Sender<Event>) -> std::io::Result<()> {


    let sock = UdpSocket::bind(
        SocketAddr::new(
            "0.0.0.0".parse().expect("what should go wrong?"), 
            2101))?;

    let mut buf = [0; 1024];

    debug!("udp_resolver");

    loop {

        let (len, addr) = sock.recv_from(&mut buf)?;
        let raw = String::from_utf8_lossy(&buf[0..len]);
        let incoming = raw.trim();

        println!("{:?} bytes received from {:?}: |{}|", len, addr, incoming);

        if incoming.starts_with("roteld {") {

            let nm = serde_json::from_str::<NetworkMessage>(&incoming[7..])
                .unwrap_or_else(|e| NetworkMessage::Error(format!("{}", e)));

            match nm {
                NetworkMessage::Volume(n) => {
                    tx.send(Event::UdpTargetVolume(n));
                },
                NetworkMessage::Power(_) => (),
                NetworkMessage::Source(_) => (),
                NetworkMessage::Error(e) => {
                    error!("NetworkMessage Error: {}", e)
                },
            }
        }

    }

}




