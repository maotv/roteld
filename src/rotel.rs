
extern crate serialport;
extern crate ws;

// #[macro_use]
// extern crate serde_derive;

//extern crate serde;
extern crate serde_json;


// use ws::{Handler, Handshake, Result, Message};
// use ws::Error as WsError;

// use serde_json::Value;


use std::thread;
use std::io::{Read, Write};

// use std::sync::mpsc;
use std::sync::mpsc::{Sender,Receiver,TryRecvError};

use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// use std::sync::mpsc::{Sender,Receiver};



// use alsa::mixer::SelemId;
// use alsa::mixer::SelemChannelId;
// use alsa_sys::snd_mixer_handle_events;
// use argparse::{ArgumentParser, Store};
// use serialport::prelude::*;

use serialport::posix::TTYPort;
use std::os::unix::io::RawFd;
use std::os::unix::prelude::*;

// use std::path::Path;


use common::Event;
use common::KeyValueRaw;

const ROTEL_VOLUME_ABSMIN: i64 = 0;
const ROTEL_VOLUME_ABSMAX: i64 = 96;


const MODE_EOL: usize = 0; // variable value is terminated with ! 
const MODE_STR: usize = 1; // variable is given as ###,some text where ### is text length


const STATE_WAITFOR: usize = 0;
const STATE_VARNAME: usize = 1;
const STATE_LENGTH:  usize = 2;
const STATE_NCHARS:  usize = 3;
const STATE_READEOL: usize = 4;
const STATE_DONE:    usize = 5;

static rotel_is_adjusting_value: AtomicBool = ATOMIC_BOOL_INIT;
static rotel_knob_timestamp_value: AtomicUsize = ATOMIC_USIZE_INIT;

pub enum RotelCommand {
    Target(i64),
    Received(i64),
    Command(String)
}


struct RotelState {

    power: bool,
}

struct UnitResponse {
    state: usize,
    count: usize,
    name:  String,
    slen:  String,
    value: String,
    raw: String
}



impl UnitResponse {

    fn new() -> UnitResponse {
        UnitResponse { state: STATE_WAITFOR, count: 0, slen: String::new(), name:  String::new(), value: String::new(), raw: String::new()  }
    }

    fn clear(&mut self) {
        self.state = STATE_WAITFOR;
        self.count = 0;
        self.name  = String::new();
        self.slen  = String::new();
        self.value = String::new();
    }

}


pub struct Rotel {

}


pub fn connect() {



}

impl Rotel {
    fn connect(send: Sender<Event>) -> () {

    }
}



fn millis_since_epoch() -> usize {
    let d = SystemTime::now().duration_since(UNIX_EPOCH).expect("SystemTime before UNIX EPOCH!");
    let s: u64 = d.as_secs() * 1000;
    let m: u64 = d.subsec_nanos() as u64 / 1_000_000;
    ((s+m-1515234056) as usize)
}

// fn duration_as_millis(d: Duration) -> usize {
//     println!("duration as millis from {}", d.as_secs());
//     let s: usize = ((d.as_secs() as usize) - 1515234056) * 1000;
//     let m: usize = d.subsec_nanos() as usize / 1_000_000;
//     s+m
// }


pub fn rotel_knob_set_timestamp() {
    rotel_knob_timestamp_value.store(millis_since_epoch(), Ordering::Relaxed)
}

pub fn rotel_knob_is_turning() -> bool {
    (millis_since_epoch() - rotel_knob_timestamp_value.load(Ordering::Relaxed)) < 3000
}


pub fn rotel_is_adjusting() -> bool {
    rotel_is_adjusting_value.load(Ordering::Relaxed)
}



pub fn rotel_command_thread(fd: RawFd, rx: Receiver<RotelCommand>) -> () {

    println!("rotel thread with fd {}", fd);
    let mut port: TTYPort = unsafe {  
         TTYPort::from_raw_fd(fd)
    };

    port.write_all("power_on!".as_bytes());
    port.flush();

    port.write_all(&"pc_usb!".as_bytes());
    port.flush();    

    port.write_all(&"get_volume!".as_bytes());
    port.flush();    

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

            Ok(RotelCommand::Target(v)) => {
                println!("    Set target: {}", v);
                rotel_volume_target = v;
                if !rotel_is_adjusting() {
                    rotel_volume_sent = rotel_volume_received; // initialize sent.
                }
                rotel_is_adjusting_value.store(true, Ordering::Relaxed);
            },

            Ok(RotelCommand::Received(v)) => {
                rotel_volume_received = v;
            },

            Ok(RotelCommand::Command(s)) => {
                println!("[rotel ] Serial Event ({})", s);
                port.write_all(&s.as_bytes());
            },

            Err(TryRecvError::Disconnected) => println!("Disconnected in rotel command thread"),
            Err(TryRecvError::Empty) => (),
            
        }

        if rotel_is_adjusting() {
            if rotel_volume_received == rotel_volume_target {

                // this is it. no mor adjusting necessary
                rotel_is_adjusting_value.store(false, Ordering::Relaxed);
                println!("    Done.");

            } else {


                if rotel_volume_sent == rotel_volume_target {

                    // no more updates needed.
                    println!("    Waiting for confirmation: sent: {} received: {} target: {}", rotel_volume_sent, rotel_volume_received, rotel_volume_target);
                    let rotel_command = format!("volume_{}!", rotel_volume_sent);
                    println!("    Send2: {}", rotel_command);
                    port.write_all(&rotel_command.as_bytes());

                } else {

                    rotel_volume_sent = rotel_volume_sent + (rotel_volume_target-rotel_volume_sent).signum();
                    let rotel_command = format!("volume_{}!", rotel_volume_sent);
                    println!("    Send1: {}", rotel_command);
                    // set on rotel device
                    port.write_all(&rotel_command.as_bytes());

                }



            }
        }
    
        thread::sleep(Duration::from_millis(30));

    }




}



pub fn rotel_reader_thread(fd: RawFd, tx: Sender<Event>) -> () {
    
    println!("rotel thread with fd {}", fd);
    let mut port: TTYPort = unsafe {  
         TTYPort::from_raw_fd(fd)
    };

    let mut ures = UnitResponse::new(); 
    let mut serial_buf: Vec<u8> = vec![0; 1000];

    loop {

        if let Ok(t) = port.read(serial_buf.as_mut_slice()) {
            
            for c in serial_buf[..t].iter() {

                process_one(&mut ures, *c);

                if ures.state == STATE_DONE  {

                    // println!("[Rotel  ] {} = {}", ures.name, ures.value);
                    tx.send(Event::Rotel( KeyValueRaw { name: ures.name, value: ures.value, raw: ures.raw } )).unwrap();
                    ures = UnitResponse::new(); 

                }
            }
        }
    }
}


pub fn parse_rotel_volume(v: &String) -> i64 {
    if v == "min" {
        ROTEL_VOLUME_ABSMIN
    } else if v == "max" {
        ROTEL_VOLUME_ABSMAX
    } else {
        v.parse().unwrap()
    }
}


fn process_one(rv: &mut UnitResponse, c: u8) {

    match rv.state {
        STATE_WAITFOR => wait_for_character(rv, c),
        STATE_VARNAME => read_var_name(rv, c),
        STATE_LENGTH  => read_length(rv, c),
        STATE_NCHARS  => read_n_chars(rv, c),
        STATE_READEOL => read_to_eol(rv, c),
        _ => ()
    }

}

fn wait_for_character(rv: &mut UnitResponse, c: u8)  {
    if  c > 32 && c < 127  {
        rv.state = STATE_VARNAME;
        read_var_name(rv, c);
    }
}


fn read_var_name(rv: &mut UnitResponse, c: u8)  {

    rv.raw.push(c as char);

    if  c as char == '='  {
        // println!("VARNAME => {} is {}", rv.name, ctype(&rv.name));
        if ctype(&rv.name) == MODE_EOL {
            rv.state = STATE_READEOL;
        } else {
            rv.state = STATE_LENGTH;
        }
    } else if c as char == '!'  {
        // WARN: invalid command name
        println!("WARN: invalid command name {}", rv.name);
    } else {
        rv.name.push(c as char);
    }

}

fn read_length(rv: &mut UnitResponse, c: u8)  {

    rv.raw.push(c as char);

    if c as char == ','  {
        let len: usize = rv.slen.parse().expect("not a number");
        // println!("LENGTH => {} = {}", rv.slen, len);
        rv.state = STATE_NCHARS;
        rv.count = len;
    } else {
        rv.slen.push(c as char);
    }

}

fn read_n_chars(rv: &mut UnitResponse, c: u8)  {

    rv.raw.push(c as char);

    if rv.count > 0 {
        rv.value.push(c as char);
        rv.count = rv.count-1;
    } 

    if rv.count == 0 {
        // println!("VALUE => {} is {}", rv.name, rv.value);
        rv.state = STATE_DONE;
    }

}

fn read_to_eol(rv: &mut UnitResponse, c: u8)  {

    rv.raw.push(c as char);

    if c as char == '!' {
        // println!("VALUE => {} is {}", rv.name, rv.value);
        rv.state = STATE_DONE;
    } else {
        rv.value.push(c as char);
    }

}


fn ctype(command: &str) -> usize {

    match command {
        "display"  => MODE_STR,
        "display1" => MODE_STR,
        "display2" => MODE_STR,
        "product_type" => MODE_STR,
        "product_version" => MODE_STR,
        "tc_version" => MODE_STR,
        "display_size" => MODE_EOL,
        "display_update" => MODE_EOL,
        "power" => MODE_EOL,
        "volume" => MODE_EOL,
        "mute" => MODE_EOL,
        "source" => MODE_EOL,
        "tone" => MODE_EOL,
        "bass" => MODE_EOL,
        "treble" => MODE_EOL,
        "balance" => MODE_EOL,
        "speaker" => MODE_EOL,
        "pcusb_class" => MODE_EOL,
        "play_status" => MODE_EOL,
        "dimmer" => MODE_EOL,
        "freq" => MODE_EOL,
        _ => MODE_EOL

    }

}


