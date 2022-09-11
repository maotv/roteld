


use log::{debug,warn,error};
// use ws::{Handler, Handshake, Result, Message};
// use ws::Error as WsError;

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

use serialport::TTYPort;
use crate::common::KeyValueRaw;

use super::RotelEvent;




enum ParserState {

    WAITFOR, // wait for a ascii (non-control-) character on the stream
    VARNAME, // read the variable name in a somename=someval! construct
    LENGTH,  // read the length of the value in name=###,text
    NCHARS,  // read n chars
    READEOL, // read until !
    DONE     // done reading one command

}


enum CmdMode {
    EOL, // variable value is terminated with ! 
    STR, // variable is given as ###,some text where ### is text length
    PWR  // special case "00:power_off!"
}

struct UnitResponse {
    state: ParserState,
    count: usize,
    name:  String,
    slen:  String,
    value: String,
    raw: String
}


impl UnitResponse {

    fn new() -> UnitResponse {
        UnitResponse { 
            state: ParserState::WAITFOR, 
            count: 0, 
            slen: String::new(), 
            name:  String::new(), 
            value: String::new(), 
            raw: String::new()  
        }
    }

}

pub fn rotel_reader_thread(fd: RawFd, tx: Sender<RotelEvent>) -> () {
    
    println!("rotel_reader_thread with fd {}", fd);

    // do nothing if there is no port exept keeping messaging open.
    if fd == 0 {
        warn!("no serial port, will idle");
        loop {
            thread::sleep(Duration::from_millis(7000));
        }
    }


    let mut port: TTYPort = unsafe {  
         TTYPort::from_raw_fd(fd)
    };

    let mut ures = UnitResponse::new(); 
    let mut serial_buf: Vec<u8> = vec![0; 1000];

    loop {

        if let Ok(t) = port.read(serial_buf.as_mut_slice()) {
            
            for c in serial_buf[..t].iter() {

                process_one(&mut ures, *c);

                if let ParserState::DONE = ures.state {
                    println!("[Rotel  ] Message: [{}] = \"{}\"", ures.name, ures.value);
                    if ures.name.len() > 1 {
                        tx.send(RotelEvent::AmpMessage( KeyValueRaw { key: ures.name, value: ures.value, raw: ures.raw } )).unwrap();
                    } else {
                        debug!("[Rotel  ] Ignoring message due to insufficient length");
                    }
                    // reset the state machine
                    ures = UnitResponse::new(); 
                }
            }

        }
    }
}



fn process_one(rv: &mut UnitResponse, c: u8) {

/*
    if  c > 32 && c < 127  {
        println!("[C] {}", c as char);
    } else {
        println!("[C] #{}", c);
    }
*/

    match rv.state {
        ParserState::WAITFOR => wait_for_character(rv, c),
        ParserState::VARNAME => read_var_name(rv, c),
        ParserState::LENGTH  => read_length(rv, c),
        ParserState::NCHARS  => read_n_chars(rv, c),
        ParserState::READEOL => read_to_eol(rv, c),
        _ => ()
    }

}

fn wait_for_character(rv: &mut UnitResponse, c: u8)  {
    if  c > 32 && c < 127  {
        rv.state = ParserState::VARNAME;
        read_var_name(rv, c);
    }
}


fn read_var_name(rv: &mut UnitResponse, c: u8)  {

    rv.raw.push(c as char);

    if  c as char == '='  {
        match ctype(&rv.name) {
            CmdMode::EOL => rv.state = ParserState::READEOL,
            CmdMode::STR => rv.state = ParserState::LENGTH,
            CmdMode::PWR => rv.state = ParserState::DONE
        }

        // println!("VARNAME => {} is {}", rv.name, ctype(&rv.name));
        // if ctype(&rv.name) == MODE_EOL {
        //     rv.state = STATE_READEOL;
        // } else {
        //     rv.state = STATE_LENGTH;
        // }
    } else if c as char == '!'  {
        // WARN: invalid command name
        // println!("WARN: invalid command name {}", rv.name);
        rv.state = ParserState::DONE;
    } else {
        rv.name.push(c as char);
    }

}

fn read_length(rv: &mut UnitResponse, c: u8)  {

    rv.raw.push(c as char);

    if c as char == ','  {
        let len: usize = rv.slen.parse().expect("not a number");
        // println!("LENGTH => {} = {}", rv.slen, len);
        rv.state = ParserState::NCHARS;
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
        rv.state = ParserState::DONE;
    }

}

fn read_to_eol(rv: &mut UnitResponse, c: u8)  {

    rv.raw.push(c as char);

    if c as char == '!' {
        // println!("VALUE => {} is {}", rv.name, rv.value);
        rv.state = ParserState::DONE;
    } else {
        rv.value.push(c as char);
    }

}


fn ctype(command: &str) -> CmdMode {

    match command {
        "00:power_off" => CmdMode::PWR,
        "00:power_on" => CmdMode::PWR,
        "power_off" => CmdMode::PWR,
        "power_on" => CmdMode::PWR,
        "display"  => CmdMode::STR,
        "display1" => CmdMode::STR,
        "display2" => CmdMode::STR,
        "product_type" => CmdMode::STR,
        "product_version" => CmdMode::STR,
        "tc_version" => CmdMode::STR,
        "display_size" => CmdMode::EOL,
        "display_update" => CmdMode::EOL,
        "power" => CmdMode::EOL,
        "volume" => CmdMode::EOL,
        "mute" => CmdMode::EOL,
        "source" => CmdMode::EOL,
        "tone" => CmdMode::EOL,
        "bass" => CmdMode::EOL,
        "treble" => CmdMode::EOL,
        "balance" => CmdMode::EOL,
        "speaker" => CmdMode::EOL,
        "pcusb_class" => CmdMode::EOL,
        "play_status" => CmdMode::EOL,
        "dimmer" => CmdMode::EOL,
        "freq" => CmdMode::EOL,
        _ => CmdMode::EOL

    }

}

