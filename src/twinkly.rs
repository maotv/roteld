

use log::{trace,info,warn,error};

use std::net::UdpSocket;

use std::sync::mpsc;
use std::thread;
use crate::common::*;

use std::time::{Duration, SystemTime, Instant, UNIX_EPOCH};
use serde_json::Value;
use serde_json::json;

use chrono::prelude::*;



pub struct Twinkly {
    host: String
}

impl Twinkly {

    pub fn new(host: &str) -> Self {
        Twinkly {
            host: String::from(host)
        }
    }

    pub fn start(&self) {
        thread::spawn(move || {
            twinkly_main_thread();
        });
    }



	// public JsonObject request(String rq, Object body) throws Exception {

    //     let resp = reqwest::blocking::get("https://httpbin.org/ip")?
    //         .json::<HashMap<String, String>>()?;


    //         let client = reqwest::Client::new();
    //         let res = client.post("http://httpbin.org/post")
    //             .body("the exact body that is sent")
    //             .send()
    //             .await?;

    //             let client = reqwest::Client::new();
    //             let res = client.post("http://httpbin.org/post")
    //                 .json(&map)
    //                 .send()
    //                 .await?;


		
	// 	//create defaultHttpClient
	// 	CloseableHttpClient httpClient = HttpClients.custom().build();

		
	// 	HttpPost httpPost = new HttpPost(rq);
	// 	if ( token != null ) httpPost.setHeader("X-Auth-Token", token);
		
	// 	HttpEntity postBody = null; // new Gson().toJson(body);
	// 	if ( body instanceof String ) {
	// 		httpPost.setHeader("Content-Type", "application/json");
	// 		postBody = new StringEntity((String)body, StandardCharsets.UTF_8);
	// 	} else if ( body instanceof byte[] ) {
	// 		httpPost.setHeader("Content-Type", "application/octet-stream");
	// 		postBody = new ByteArrayEntity((byte[])body);
	// 	}
		
		
		
	// 	if ( postBody != null ) httpPost.setEntity(postBody);
		
	// 	System.err.println("Post: " + httpPost.getURI() + ": " + postBody);
	// 	HttpResponse resp = httpClient.execute(httpPost);
		
	// 	System.err.println("Resp: " + resp);
		
		
	// 	HttpEntity ent = resp.getEntity();
	// 	if ( ent != null ) {
	// 		InputStream in = resp.getEntity().getContent();
	// 		InputStreamReader isr = new InputStreamReader(in, StandardCharsets.UTF_8);
	// 		String json = IOUtils.toString(isr);
	// 		System.err.println(json);
	// 		JsonObject jo = new JsonParser().parse(json).getAsJsonObject();
	// 		httpClient.close();
	// 		return jo;
			
	// 	}
		
	// 	return null;
		
	// }
	
}



fn request(base: &str, path: &str, token: &str, body: Value) -> Result<Value,reqwest::Error> {


    let client = reqwest::blocking::Client::new();
    let url = format!("{}{}", base, path);

    info!("==== Request ({}) {}", token, url);

    let resp = client.post(&url)
        .header("X-Auth-Token",token)
        .header("Content-Type", "application/json")
        .json(&body)
        .send();

    info!("response1: {:?}", resp);

    let json = resp.unwrap().json();
    info!("response2: {:?}", json);

    json
}



pub fn twinkly_main_thread() {


    for i in 0..250 {
        print!("{}, ", i);
    }

    println!("");
    for i in 0..250 {
        print!("{}, ", 250-i);
    }

    println!("");

    let host = "10.12.67.107";
    let base = format!("http://{}/xled/v1", host);

    let socket = UdpSocket::bind("0.0.0.0:3401").unwrap();

    let mut token = String::new();
    

    if let Ok(js) = request(&base, "/login", "", json!({
        "challenge": "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8="
    })) {
        if let Some(tok) = js.get("authentication_token") {
            token = String::from(tok.as_str().unwrap_or(""));
            info!("===== Token is {}", token);
        }
    }

    let vv = request(&base, "/verify", &token, json!({}));
    info!("verify: {:?}", vv);

    let gs = request(&base, "/gestalt", &token, json!({}));
    info!("gestalt: {:?}", gs);

    let rt = request(&base, "/led/mode", &token, json!({ "mode": "rt"}));
    info!("rt: {:?}", rt);
    

    let toaddr = format!("{}:7777", host);

    let tokdec = base64::decode(&token).unwrap();

    let mut data:[u8; 760] = [0; 760];
    data[0] = 1;
    data[1] = tokdec[0];
    data[2] = tokdec[1];
    data[3] = tokdec[2];
    data[4] = tokdec[3];
    data[5] = tokdec[4];
    data[6] = tokdec[5];
    data[7] = tokdec[6];
    data[8] = tokdec[7];
    data[9] = 250;



    socket.send_to(&data, &toaddr).expect("couldn't send data");


    // thread::sleep(Duration::from_millis(7000));

    // let mv = request(&base, "/led/mode", &token, json!({ "mode": "movie"}));
    // info!("mv: {:?}", mv);

    let startTime = Instant::now();

    let mut beep = BeepBeep { state: 0 };
    let mut delay = Duration::from_millis(1000);
    loop {

        info!("show...");

        match beep.frame(Instant::now().duration_since(startTime).as_secs_f64()) {
            Frame::Show(frm,dly) => {
                delay = Duration::from_millis(dly);
                for  i in 0..frm.len() {
                    data[i+10] = frm[i];
                    socket.send_to(&data, &toaddr).expect("couldn't send data");
                }


            },
            Frame::Wait(dly) => {
                delay = Duration::from_millis(dly);

            }
        }

        thread::sleep(delay);

    }



    // JsonObject login = request(BASEURL + "/login", "{\"challenge\": \"AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=\"}");
    // token = login.get("authentication_token").getAsString();
    // tokdec = Base64.getDecoder().decode(token);


	// 	request(BASEURL + "/verify", "{ }");
	// 	get(BASEURL + "/device_name");



}


enum Frame {
    Show(Vec<u8>, u64),
    Wait(u64)
}

trait FilmRoll {
    fn frame(&mut self, time: f64) -> Frame; 
}




struct BeepBeep { state: usize }

impl BeepBeep {

}


impl FilmRoll for BeepBeep {
    fn frame(&mut self, _time: f64) -> Frame {

        let mut fnull: Vec<u32> = vec!(0;250);

       // match self.state {
        //     0 => { fnull[16] = 0xff000000; self.state = 1 },
        //     1 => { fnull[16] = 0x00000000; self.state = 0 },
        //     _ => self.state = 0
        // } ;

        let hand: Vec<usize> = vec![

            7,8,9,10,11,12,13,14,
            29,44,59,74,89,104,119,119,

            134,149,164,179,194,209,224,
            223,222,221,220,219,218,217,217,
            
            216,215,214,213,212,211,210,195,180,165,150,135,120,105,105,
            90,75,60,45,30,15,0,1,2,3,4,5,6,7

        ]; 


        let now = Local::now();
        let hors = now.hour();
        let mins = now.minute();
        let secs = now.second();

        let next = match now.nanosecond() > 250_000_000 {
            true => 900,
            false => 1100
        };

        let dot = hand[secs as usize];
        fnull[dot] = 0xff000000;

        let h10 = (hors/10) as usize;
        let h01 = hors as usize - (h10*10);

        let m10 = (mins/10) as usize;
        let m01 = mins as usize - (m10*10);


        blit8(&mut fnull, 33-4, 15, &SMALLDIGITS, h10*5, 5);
        blit8(&mut fnull, 38-4, 15, &SMALLDIGITS, h01*5, 5);

        blit8(&mut fnull, 123-4, 15, &SMALLDIGITS, m10*5, 5);
        blit8(&mut fnull, 128-4, 15, &SMALLDIGITS, m01*5, 5);




        self.state += 1;
        if self.state > 224 { self.state = 0 }

        let ba = to_byte_array(fnull);
        Frame::Show(ba,next)
    }
}

fn to_byte_array(pixels: Vec<u32>) -> Vec<u8> {

    let mut out: Vec<u8> = vec!(0;750);
//    let mut o2: [u8; 250] = [0;250];

    // let pm = pixelmap();
    for i in 0..225 {
        let pix = pixels[i].to_be_bytes();
        let dest = SCREENMAP[i];
        if dest < 250 {
            out[dest*3]   = pix[0];
            out[dest*3+1] = pix[1];
            out[dest*3+2] = pix[2];
        } else {
            warn!("WTF {}", dest);
        }
    }

    out
}

fn blit8(map: &mut Vec<u32>, origin: usize, stride: usize, font: &[u8], offset: usize, lines: usize ) {

    for y in 0..lines {
        for x in 0..8 {
            let index = origin + (y*stride) + x;
//            map[index] = 0x00ffff00;
           //  map[index+x] = 0x0000ff00;
            if font[offset+y] & (0b10000000u8 >> x) > 0 {
                if index < map.len() {
                    map[index] = 0x0000ff00;
                } 
            }
        }
    }

}





const SMALLDIGITS: [u8;50] = [
    
    0b1111,
    0b1001,
    0b1001,
    0b1001,
    0b1111,

    0b0010,
    0b0010,
    0b0010,
    0b0010,
    0b0010,

    0b1111,
    0b0001,
    0b1111,
    0b1000,
    0b1111,

    0b1111,
    0b0001,
    0b1111,
    0b0001,
    0b1111,

    0b1000,
    0b1000,
    0b1010,
    0b1111,
    0b0010,

    0b1111,
    0b1000,
    0b1111,
    0b0001,
    0b1111,

    0b1111,
    0b1000,
    0b1111,
    0b1001,
    0b1111,

    0b1111,
    0b0001,
    0b0001,
    0b0001,
    0b0001,

    0b1111,
    0b1001,
    0b1111,
    0b1001,
    0b1111,

    0b1111,
    0b1001,
    0b1111,
    0b0001,
    0b1111,

];

const SCREENMAP: [usize;225] = [
    124, 123, 122, 121, 120, 119, 118, 117, 116, 115, 114, 113, 112, 111, 110,
    95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109,
    94, 93, 92, 91, 90, 89, 88, 87, 86, 85, 84, 83, 82, 81, 80, 
    65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79,
    64, 63, 62, 61, 60, 59, 58, 57, 56, 55, 54, 53, 52, 51, 50, 
    35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    34, 33, 32, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 
    5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 
    4, 3, 2, 1, 0, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240, 
    225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239,
    224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210,
    195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 
    194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180,
    165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179,
    164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150 ];
