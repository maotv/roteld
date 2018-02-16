extern crate argparse;
extern crate serialport;
// extern crate alsa;
// extern crate alsa_sys;

extern crate ws;

#[macro_use]
extern crate serde_derive;

//extern crate serde;
extern crate serde_json;


use ws::{Handler, Handshake, Result, Message};
use ws::Error as WsError;

use serde_json::Value;


use std::thread;
use std::io::{self, Read, Write};

use std::sync::mpsc;
use std::sync::mpsc::{Sender,Receiver,TryRecvError};

use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// use std::sync::mpsc::{Sender,Receiver};



// use alsa::mixer::SelemId;
// use alsa::mixer::SelemChannelId;
// use alsa_sys::snd_mixer_handle_events;
// use argparse::{ArgumentParser, Store};
use serialport::prelude::*;

use serialport::posix::TTYPort;
use std::os::unix::io::RawFd;
use std::os::unix::prelude::*;

use std::path::Path;

// const EMPTY: &'static str = "";

const ROTEL_VOLUME_MIN: i64 = 1;
const ROTEL_VOLUME_MAX: i64 = 64;

const VOLUMIO_VOLUME_MIN: i64 = 0;
const VOLUMIO_VOLUME_MAX: i64 = 100;

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

enum RotelCommand {
    Target(i64),
    Received(i64),
    Command(String)
}

// struct VolumioState {
//     volume: i64,
// }

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




struct RotelState {

    power: bool,
}

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



struct UnitResponse {
    state: usize,
    count: usize,
    name:  String,
    slen:  String,
    value: String
}

impl UnitResponse {

    fn new() -> UnitResponse {
        UnitResponse { state: STATE_WAITFOR, count: 0, slen: String::new(), name:  String::new(), value: String::new()  }
    }

    fn clear(&mut self) {
        self.state = STATE_WAITFOR;
        self.count = 0;
        self.name  = String::new();
        self.slen  = String::new();
        self.value = String::new();
    }

}


enum Event {

    Rotel(UnitResponse),
//    Volumio(VolumioState),
    Volumio(Value),
    WsConnect(ws::Sender),
    WsPing

}




// Our Handler struct.
// Here we explicity indicate that the Client needs a Sender,
// whereas a closure captures the Sender for us automatically.
struct WsClient {
    out: ws::Sender,
    tx:  mpsc::Sender<Event>,
}


// We implement the Handler trait for Client so that we can get more
// fine-grained control of the connection.
impl Handler for WsClient {

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


fn rotel_knob_set_timestamp() {
    rotel_knob_timestamp_value.store(millis_since_epoch(), Ordering::Relaxed)
}

fn rotel_knob_is_turning() -> bool {
    (millis_since_epoch() - rotel_knob_timestamp_value.load(Ordering::Relaxed)) < 3000
}


fn main() {


    let port_name = "/dev/ttyUSB0";

    let settings = SerialPortSettings {

        baud_rate: BaudRate::Baud115200,
        data_bits: DataBits::Eight,
        flow_control: FlowControl::None,
        parity: Parity::None,
        stop_bits: StopBits::One,
        timeout: Duration::from_millis(1),

    };


//    let (tx_v, rx_v) = mpsc::channel();
    let (tx_main, rx_main) = mpsc::channel();
    let tx_main_r = tx_main.clone();
    let tx_main_ping = tx_main.clone();
    //let tx2 = mpsc::Sender::clone(&tx1);

    let (tx_command, rx_command) = mpsc::channel();


    if let Ok(mut port) = TTYPort::open(Path::new(port_name), &settings) {

        let fd_read  = port.as_raw_fd();
        // let fd_write = port.as_raw_fd(); // clone??

        println!("port is open! #{}", fd_read);

        thread::spawn(move || {
            rotel_reader_thread(fd_read, tx_main_r);
        });

        thread::spawn(move || {
            rotel_command_thread(fd_read, rx_command);
        });

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(25000));
                tx_main_ping.send(Event::WsPing);
            }
        });


        thread::spawn(move || {

            loop {

                println!("[Setup  ] Client Connect to Volumio Websocket");
                ws::connect("ws://127.0.0.1:3000/socket.io/?EIO=3&transport=websocket", |out| WsClient { out: out, tx: tx_main.clone() } ).unwrap();
                println!("[Setup  ] Client Connection closed");
                thread::sleep(Duration::from_millis(300));
            }

        });

       // let t_alsa = thread::spawn(move || {
       //      main_alsa_thread(fd_write, rx_r, rx_v);
       //  });


        let mut volumio_current_volume: i64 = 0;
        // let mut rotel_current_volume: i64 = 0;
        // let mut rotel_target_volume: i64  = 0;

        let mut volumio_sender: Option<ws::Sender> = None;

        loop {

            match rx_main.recv() {

                Ok(Event::Rotel(ur)) => {
                    // println!("[Main   ] Rotel Event: {}", ur.name );

                    if ur.name == "volume" {

                        let rotvol = parse_rotel_volume(&ur.value);
                        // println!("[Main   ] Rotel Event: Volume {}", rotvol);
                        tx_command.send(RotelCommand::Received(rotvol));

                        if !rotel_is_adjusting() {
                            rotel_knob_set_timestamp();
                        }

                        if rotel_knob_is_turning() {
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

                Ok(Event::Volumio(ps)) => {
                    
                    let ps_volume: i64 = ps["volume"].as_i64().unwrap();

                    if ps_volume != volumio_current_volume {

                        volumio_current_volume = ps_volume;

                        if rotel_knob_is_turning() { // n second timeout when directly setting rotel volume
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

                Ok(Event::WsConnect(snd)) => {
                    volumio_sender = Some(snd);
                },

                Ok(Event::WsPing) => {
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



//        t_rotel.join();
//        t_alsa.join(); // just in case
        
    }



}








fn rotel_is_adjusting() -> bool {
    rotel_is_adjusting_value.load(Ordering::Relaxed)
}

fn rotel_command_thread(fd: RawFd, rx: Receiver<RotelCommand>) -> () {

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
                println!("NYI {}", s)
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



fn rotel_reader_thread(fd: RawFd, tx: Sender<Event>) -> () {
    
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
                    tx.send(Event::Rotel(ures)).unwrap();
                    ures = UnitResponse::new(); 

                }
            }
        }
    }
}


fn normal_volume(min: i64, max: i64, value: i64) -> f64 {
    ((value - min) as f64 / (max - min) as f64).max(0.0).min(1.0)
}

fn device_volume(min: i64, max: i64, value: f64) ->i64 {
    ((value  * (max-min) as f64) + min as f64 ) as i64
}

fn parse_rotel_volume(v: &String) -> i64 {
    if v == "min" {
        ROTEL_VOLUME_MIN
    } else if v == "max" {
        ROTEL_VOLUME_MAX
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




