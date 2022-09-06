
extern crate serialport;
extern crate ws;


// #[macro_use]
// extern crate serde_derive;

//extern crate serde;
extern crate serde_json;


// mod rwc;
mod protocol;

use log::{trace,info,warn,error, debug};
use serde_json::json;
// use ws::{Handler, Handshake, Result, Message};
// use ws::Error as WsError;

use std::net::{UdpSocket, SocketAddr};
// use serde_json::Value;
use std::sync::mpsc;

use std::thread;
use std::io::{Read, Write};

// use std::sync::mpsc;
use std::sync::mpsc::{Sender,Receiver,TryRecvError};

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, Instant, UNIX_EPOCH};

// use std::sync::mpsc::{Sender,Receiver};

use std::path::Path;


// use alsa::mixer::SelemId;
// use alsa::mixer::SelemChannelId;
// use alsa_sys::snd_mixer_handle_events;
// use argparse::{ArgumentParser, Store};
// use serialport::prelude::*;

// use std::os::unix::io::RawFd;
// use std::os::unix::prelude::*;

// use std::path::Path;
use serialport::{SerialPortBuilder, StopBits, DataBits, FlowControl, Parity};
use serialport::TTYPort;


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

// static ROTEL_POWER_STATE:   AtomicUsize = AtomicUsize::new(0); //  ATOMIC_BOOL_INIT;

// static ROTEL_IS_ADJUSTING_VALUE:   AtomicBool = AtomicBool::new(false); //  ATOMIC_BOOL_INIT;
// static ROTEL_KNOB_TIMESTAMP_VALUE: AtomicUsize = AtomicUsize::new(0); // ATOMIC_USIZE_INIT;

const ROTEL_VOLUME_ABSMIN: i64 = 0;
const ROTEL_VOLUME_ABSMAX: i64 = 96;

const ROTEL_VOLUME_LIMIT: i64 = 72;

type VolumeTarget = i64;

#[derive(Debug)]
pub enum RotelEvent {
    VolumeTarget(f64),     // from volumio, this is the target volume level
    VolumeAdjustmentRequest(i64), // from smmoth volume thread, forward to the amp
    VolumeAdjustmentDone,
    Response(KeyValueRaw), // from the amp
    Command(String),

    // Target(i64),
    // PowerState(usize),
    // Received(i64),
    // Command(String)
}

pub enum SmoothVolume {
    Adjust(i64, i64), // from, to
    Current(i64),
    Ignore
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



struct RotelState {

    device_volume: i64,
    target_volume: i64,
    is_adjusting: bool,
    knob_timestamp: SystemTime,


}

impl RotelState {

    fn new() -> Self {
        RotelState {
            device_volume:  0,
            target_volume : 0,
            is_adjusting: false,
            knob_timestamp: SystemTime::now()
        }
    }

    fn knob_touch(&mut self) {
        self.knob_timestamp = SystemTime::now();
    }

    fn knob_is_turning(&self) -> bool { 
        if let Ok(d) = SystemTime::now().duration_since(self.knob_timestamp) {
            d.as_millis() < 3000
        } else {
            false
        }
    }

}

// pub fn rotel_knob_set_timestamp() {
//     ROTEL_KNOB_TIMESTAMP_VALUE.store(millis_since_epoch(), Ordering::Relaxed)
// }

// pub fn rotel_knob_is_turning() -> bool {
//     (millis_since_epoch() - ROTEL_KNOB_TIMESTAMP_VALUE.load(Ordering::Relaxed)) < 3000
// }


// pub fn rotel_is_adjusting() -> bool {
//     ROTEL_IS_ADJUSTING_VALUE.load(Ordering::Relaxed)
// }



pub struct RotelDevice {
    tty: Option<TTYPort>
}


impl RotelDevice {

    pub fn new(serial: &str) -> RotelDevice {

        let port_name = String::from(serial); "/dev/ttyUSB0";

        // let settings = SerialPortSettings {

        //     baud_rate: 115200, // BaudRate::Baud115200,
        //     data_bits: DataBits::Eight,
        //     flow_control: FlowControl::None,
        //     parity: Parity::None,
        //     stop_bits: StopBits::One,
        //     timeout: Duration::from_millis(1),
        // };

        let spb = serialport::new(port_name,115200)
            .data_bits(DataBits::Eight)
            .flow_control(FlowControl::None)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .timeout(Duration::from_millis(1));

        let optty = TTYPort::open(&spb).ok();
        RotelDevice { tty: optty }
        
    }
 
    
    pub fn start(&mut self, tx: Sender<Event>) -> Sender<RotelEvent> {

        // workaround for rust-analyzer #6038
        use std::os::unix::io::AsRawFd;

        let port = match &self.tty {
            Some(p) => p.as_raw_fd(),
            None => 0
        };


        let (revt_tx, revt_rx) = mpsc::channel();

        // let port = self.tty.unwrap_or(0).as_raw_fd();
        println!("port is open! #{}", port);


        let (smooth_tx,smooth_rx) = mpsc::channel();
        let smooth_to_main =  revt_tx.clone();

        thread::spawn(move || {
            rotel_smooth_volume_thread(smooth_to_main, smooth_rx);
        });

        let revt_tx2 =  revt_tx.clone();
        thread::spawn(move || {
            protocol::rotel_reader_thread(port,revt_tx2);
        });

        thread::spawn(move || {
            rotel_main_thread(port, tx, revt_rx, smooth_tx);
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



fn write_command(port: &mut TTYPort, cmd: &str) {
    if let Err(e) = port.write_all(cmd.as_bytes()).and_then(|_| port.flush()) {
        warn!("cannot write to port: {:?}", e)
    }
}


#[derive(Serialize,Deserialize)]
pub struct VolumeBroadcast {
    device: usize,
    volume: usize
}




pub fn rotel_main_thread(fd: std::os::unix::io::RawFd, tx: Sender<Event>, rx: Receiver<RotelEvent>, to_smooth: Sender<SmoothVolume>) -> () {

    
    println!("rotel_command_thread with fd {}", fd);

    let mut state = RotelState::new();



    // do nothing without port
    if fd == 0 {
        loop {
            match rx.try_recv() {
                Ok(ev) => println!("    (Dummy) Event: {:?}", ev),
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) =>  println!("    (Dummy) Disconnected")
            }
            thread::sleep(Duration::from_millis(100));
        }
    }


    let mut sock = UdpSocket::bind(
        SocketAddr::new(
            "0.0.0.0".parse().expect("what should go wrong?"), 
            2102));


    if let Ok(s) = &mut sock {
        s.set_broadcast(true);
        s.connect( SocketAddr::new(
            "255.255.255.255".parse().expect("what should go wrong?"), 
            2100));
    }



    let mut port: TTYPort = unsafe {  
        // workaround for rust-analyzer #6038
        use std::os::unix::io::FromRawFd;
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

    // // last volume value sent to rotel
    // let mut rotel_volume_sent     = 0;
    // // 
    // let mut rotel_volume_received = 0;
    // // 
    // let mut rotel_volume_target   = 0;
    //
    // let mut rotel_is_adjusting    = false;

    loop {
    
        match rx.try_recv() {

            // set a new volume target and start adjusting
            Ok(RotelEvent::VolumeTarget(v)) => {

                println!("    Set target: {}", v);
                state.target_volume = device_volume(ROTEL_VOLUME_ABSMIN, ROTEL_VOLUME_LIMIT, v);
                state.is_adjusting = true;
                to_smooth.send(SmoothVolume::Adjust(state.device_volume, state.target_volume));

            },

            Ok(RotelEvent::VolumeAdjustmentRequest(v)) => {
                info!("---------------------- vol adjust start");
                let cmd = format!("volume_{}!", v);
                write_command(&mut port, &cmd);
            },

            Ok(RotelEvent::VolumeAdjustmentDone) => {
                info!("---------------------- vol adjust done");
                state.is_adjusting = false;
            },


            // response/message from amplifier
            Ok(RotelEvent::Response(p)) => {

                match p.key.as_str() {
                    "display" => (),

                    "volume" => {

                        let rotvol = parse_rotel_volume(&p.value);
                        state.device_volume = rotvol;
                        // println!("[Rotel  ] Rotel Volume {}", rotvol);
                        // check(tx_command.send(RotelEvent::Received(rotvol)));

                        if let Ok(udp) = &sock {
                            debug!("dend to udp...");
                            let vb = serde_json::to_vec(&json!(
                                {
                                    "device": 1,
                                    "volume": rotvol
                                }
                            ));
                            if let Ok(buf) = vb {
                                udp.send(&buf);
                            }
                        }


                        if state.is_adjusting {
                            to_smooth.send(SmoothVolume::Current(rotvol));
                        } else {
                            let vnorm = normal_volume(ROTEL_VOLUME_ABSMIN, ROTEL_VOLUME_LIMIT, rotvol);
                            tx.send(Event::RotelNormVolume(vnorm));
                        }


                        // if state.knob_is_turning() {
                        //     println!("[Main   ] (set.) Rotel => Volumio {}", rotvol);
                        //     //rotel_target_volume  = rvr;
                        //     //rotel_current_volume = rvr;
                        //     let vnorm = normal_volume(ROTEL_VOLUME_ABSMIN, ROTEL_VOLUME_LIMIT, rotvol);
                        //     tx.send(Event::RotelNormVolume(vnorm));

                        // } else {
                        //     // ??? rotel_current_volume = rotvol;
                        //     println!("[Rotel  ] (ign.) Rotel => Volumio {}", rotvol);
                        // }




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

    }

}



pub fn rotel_smooth_volume_thread(tx: Sender<RotelEvent>, rx: Receiver<SmoothVolume>) {

    let mut rotel_volume_target   = 0;
    let mut rotel_volume_sent     = 0;
    let mut rotel_volume_received = 0;

    loop {

        let recv = rx.recv().unwrap_or(SmoothVolume::Ignore);
        if let SmoothVolume::Adjust(current, next_target) = recv {

            info!("Starting smooth adjustment from {} to {}", current, next_target);

            let adjustment_start = Instant::now();
            rotel_volume_target = next_target;
            rotel_volume_received = current;
            rotel_volume_sent = current;

            while  rotel_volume_received != rotel_volume_target 
                && Instant::now().duration_since(adjustment_start).as_millis() < 3000 {
    
                if rotel_volume_sent > rotel_volume_target {
                    rotel_volume_sent -= 1;
                    trace!("Dn Adjustment => {}", rotel_volume_sent);
                    if let Err(e) = tx.send(RotelEvent::VolumeAdjustmentRequest(rotel_volume_sent)) {
                        warn!("cannot send VolumeAdjustmentRequest -");
                    }
                } else if rotel_volume_sent < rotel_volume_target {
                    rotel_volume_sent += 1;
                    trace!("Up Adjustment => {}", rotel_volume_sent);
                    if let Err(e) = tx.send(RotelEvent::VolumeAdjustmentRequest(rotel_volume_sent)) {
                        warn!("cannot send VolumeAdjustmentRequest +");
                    }
                } else {
                    trace!("No Adjustment @ {}", rotel_volume_sent);
                }

                // check for new events
                match rx.try_recv().unwrap_or(SmoothVolume::Ignore) {
                    SmoothVolume::Adjust(c,v) => rotel_volume_target   = v,
                    SmoothVolume::Current(v)  => rotel_volume_received = v,
                    _ => ()
                }

                thread::sleep(Duration::from_millis(40));
            }
        
            tx.send(RotelEvent::VolumeAdjustmentDone);

        } else {
            warn!("should not happen ???");
            thread::sleep(Duration::from_millis(3000));
        }


    }


}



