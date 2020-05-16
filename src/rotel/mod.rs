
extern crate serialport;
extern crate ws;


// #[macro_use]
// extern crate serde_derive;

//extern crate serde;
extern crate serde_json;


// mod rwc;
mod protocol;

use log::{debug,warn,error};
// use ws::{Handler, Handshake, Result, Message};
// use ws::Error as WsError;

// use serde_json::Value;
use std::sync::mpsc;

use std::thread;
use std::io::{Read, Write};

// use std::sync::mpsc;
use std::sync::mpsc::{Sender,Receiver,TryRecvError};

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// use std::sync::mpsc::{Sender,Receiver};

use std::path::Path;


// use alsa::mixer::SelemId;
// use alsa::mixer::SelemChannelId;
// use alsa_sys::snd_mixer_handle_events;
// use argparse::{ArgumentParser, Store};
// use serialport::prelude::*;

use std::os::unix::io::RawFd;
use std::os::unix::prelude::*;

// use std::path::Path;
use serialport::prelude::*;
use serialport::posix::TTYPort;


use crate::common::*;



// const MODE_EOL: usize = 0; // variable value is terminated with ! 
// const MODE_STR: usize = 1; // variable is given as ###,some text where ### is text length


// const STATE_WAITFOR: usize = 0;
// const STATE_VARNAME: usize = 1;
// const STATE_LENGTH:  usize = 2;
// const STATE_NCHARS:  usize = 3;
// const STATE_READEOL: usize = 4;
// const STATE_DONE:    usize = 5;

const POWER_STATE_OFF: usize = 0;
const POWER_STATE_STANDBY: usize = 1;
const POWER_STATE_ON: usize = 2;

static ROTEL_POWER_STATE:   AtomicUsize = AtomicUsize::new(0); //  ATOMIC_BOOL_INIT;

static ROTEL_IS_ADJUSTING_VALUE:   AtomicBool = AtomicBool::new(false); //  ATOMIC_BOOL_INIT;
static ROTEL_KNOB_TIMESTAMP_VALUE: AtomicUsize = AtomicUsize::new(0); // ATOMIC_USIZE_INIT;

const ROTEL_VOLUME_ABSMIN: i64 = 0;
const ROTEL_VOLUME_ABSMAX: i64 = 96;

const ROTEL_VOLUME_LIMIT: i64 = 72;




pub enum RotelEvent {
    VolumeTarget(f64),
    Response(KeyValueRaw),
    Command(String),

    // Target(i64),
    // PowerState(usize),
    // Received(i64),
    // Command(String)
}


pub enum RotelMessage {


}




/*
struct RotelState {
    power: bool,
}
*/






fn parse_rotel_volume(v: &String) -> i64 {
    if v == "min" {
        ROTEL_VOLUME_ABSMIN
    } else if v == "max" {
        ROTEL_VOLUME_ABSMAX
    } else {
        v.parse().unwrap()
    }
}



pub struct RotelDevice {
    tty: Option<TTYPort>
}


impl RotelDevice {

    pub fn new(serial: &str) -> RotelDevice {

        let port_name = String::from(serial); "/dev/ttyUSB0";

        let settings = SerialPortSettings {

            baud_rate: 115200, // BaudRate::Baud115200,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_millis(1),

        };


        let optty = TTYPort::open(Path::new(&port_name), &settings).ok();
        RotelDevice { tty: optty }
        
    }


    // pub fn check(&self) {
    //     let port = self.tty.as_raw_fd();
    //     println!("port check! #{}", port);
    // }

 
    
    pub fn start(&mut self, tx: Sender<Event>, rx: Receiver<RotelEvent>) -> Sender<RotelEvent> {

        let port = match &self.tty {
            Some(p) => p.as_raw_fd(),
            None => 0
        };


        let (revt_tx, revt_rx) = mpsc::channel();

        // let port = self.tty.unwrap_or(0).as_raw_fd();
        println!("port is open! #{}", port);


        let revt_tx2 =  revt_tx.clone();
        thread::spawn(move || {
            protocol::rotel_reader_thread(port,revt_tx2);
        });


        thread::spawn(move || {
            rotel_main_thread(port, tx, revt_rx);
        });


        revt_tx

    }

}

fn millis_since_epoch() -> usize {
    let d = SystemTime::now().duration_since(UNIX_EPOCH).expect("SystemTime before UNIX EPOCH!");
    let s: u64 = d.as_secs() * 1000;
    let m: u64 = d.subsec_nanos() as u64 / 1_000_000;
    ((s+m-1515234056) as usize)
}


pub fn rotel_knob_set_timestamp() {
    ROTEL_KNOB_TIMESTAMP_VALUE.store(millis_since_epoch(), Ordering::Relaxed)
}

pub fn rotel_knob_is_turning() -> bool {
    (millis_since_epoch() - ROTEL_KNOB_TIMESTAMP_VALUE.load(Ordering::Relaxed)) < 3000
}


pub fn rotel_is_adjusting() -> bool {
    ROTEL_IS_ADJUSTING_VALUE.load(Ordering::Relaxed)
}

fn write_command(port: &mut TTYPort, cmd: &str) {
    if let Err(e) = port.write_all(cmd.as_bytes()).and_then(|_| port.flush()) {
        warn!("cannot write to port: {:?}", e)
    }
}


pub fn rotel_main_thread(fd: RawFd, tx: Sender<Event>, rx: Receiver<RotelEvent>) -> () {

    println!("rotel_command_thread with fd {}", fd);

    // do nothing without port
    if fd == 0 {
        loop {
            match rx.try_recv() {
                Ok(RotelEvent::VolumeTarget(v)) => println!("    (Dummy) Set target: {}", v),
                // Ok(RotelEvent::PowerState(v)) => println!("    (Dummy) Power: {}", v),
                Ok(RotelEvent::Response(v)) => println!("    (Dummy) Response: {} => {}", v.key, v.value),
                Ok(RotelEvent::Command(s)) => println!("    (Dummy) Command Event ({})", s),
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) =>  println!("    (Dummy) Disconnected")
            }
            thread::sleep(Duration::from_millis(100));
        }
    }


    let mut port: TTYPort = unsafe {  
         TTYPort::from_raw_fd(fd)
    };

    thread::sleep(Duration::from_millis(1000));

    write_command(&mut port, "display_update_manual!");
    // write_command(&mut port, "get_product_type!");
    write_command(&mut port, "get_current_power!");
    // write_command(&mut port, "get_volume!");

// //    port.write_all("power_on!".as_bytes()).and_then(|_| port.flush()).unwrap_or(());
// //    port.write_all(&"pc_usb!".as_bytes()).and_then(|_| port.flush()).unwrap_or(());
//     port.write_all(&"get_product_type!".as_bytes()).and_then(|_| port.flush()).unwrap_or(());
//     port.write_all(&"get_current_power!".as_bytes()).and_then(|_| port.flush()).unwrap_or(());
//     port.write_all(&"get_volume!".as_bytes()).and_then(|_| port.flush()).unwrap_or(());

    // last volume value sent to rotel
    let mut rotel_volume_sent     = 0;
    // 
    let mut rotel_volume_received = 0;
    // 
    let mut rotel_volume_target   = 0;
    //
    // let mut rotel_is_adjusting    = false;

    loop {
    
        match rx.try_recv() {

            // set a new volume target and start adjusting
            Ok(RotelEvent::VolumeTarget(v)) => {
                println!("    Set target: {}", v);

                rotel_volume_target = device_volume(ROTEL_VOLUME_ABSMIN, ROTEL_VOLUME_LIMIT, v);
                if !rotel_is_adjusting() {
                    rotel_volume_sent = rotel_volume_received; // initialize sent.
                }
                ROTEL_IS_ADJUSTING_VALUE.store(true, Ordering::Relaxed);
            },

            // response/message from amplifier
            Ok(RotelEvent::Response(p)) => {
                
                match p.key.as_str() {
                    "display" => (),

                    "volume" => {

                        let rotvol = parse_rotel_volume(&p.value);
                        // println!("[Main   ] Rotel Event: Volume {}", rotvol);
                        // check(tx_command.send(RotelEvent::Received(rotvol)));

                        if !rotel_is_adjusting() {
                            rotel_knob_set_timestamp();
                        }

                        if rotel_knob_is_turning() {
                            println!("[Main   ] (set.) Rotel => Volumio {}", rotvol);
                            //rotel_target_volume  = rvr;
                            //rotel_current_volume = rvr;
                            let vnorm = normal_volume(ROTEL_VOLUME_ABSMIN, ROTEL_VOLUME_LIMIT, rotvol);
                            tx.send(Event::RotelNormVolume(vnorm));

                        } else {
                            // ??? rotel_current_volume = rotvol;
                            println!("[Main   ] (ign.) Rotel => Volumio {}", rotvol);
                        }




                    },
                    _ => ()
                }


//                 println!("    Power State: {}", p);
                // rotel_volume_received = v;
            },

            // Ok(RotelEvent::Received(v)) => {
            //     rotel_volume_received = v;
            // },

            Ok(RotelEvent::Command(s)) => {
                println!("[rotel ] Command Event ({})", s);
                if let Err(e) = port.write_all(&s.as_bytes()) {
                    error!("[rotel ] Error ({:?})", e);
                }
            },

            Err(TryRecvError::Disconnected) => println!("Disconnected in rotel command thread"),
            Err(TryRecvError::Empty) => (),
            
        }

        if rotel_is_adjusting() {
            if rotel_volume_received == rotel_volume_target {

                // this is it. no mor adjusting necessary
                ROTEL_IS_ADJUSTING_VALUE.store(false, Ordering::Relaxed);
                println!("    Done.");

            } else {


                if rotel_volume_sent == rotel_volume_target {

                    // no more updates needed.
                    println!("    Waiting for confirmation: sent: {} received: {} target: {}", rotel_volume_sent, rotel_volume_received, rotel_volume_target);
                    let rotel_command = format!("volume_{}!", rotel_volume_sent);
                    println!("    Send2: {}", rotel_command);
                    if let Err(e) = port.write_all(&rotel_command.as_bytes()) {
                        println!("    Send2: Error ({:?})", e);
                    }

                } else {

                    rotel_volume_sent = rotel_volume_sent + (rotel_volume_target-rotel_volume_sent).signum();
                    let rotel_command = format!("volume_{}!", rotel_volume_sent);
                    println!("    Send1: {}", rotel_command);
                    // set on rotel device
                    if let Err(e) = port.write_all(&rotel_command.as_bytes()) {
                        println!("    Send1: Error ({:?})", e);
                    }

                }



            }
        }
    
        thread::sleep(Duration::from_millis(30));

    }




}




