use std::{env, io::Read, io::Write};
use std::fs::File;
use getopts::Options;
use sha2::{Sha256, Digest};
use image::{ImageFormat, RgbImage};

//code for PRNG adapted from https://prng.di.unimi.it/xoshiro256plusplus.c
struct PrngState(u64, u64, u64, u64);

fn rotl(x: u64, k: u64) -> u64 {
    (x << k) | (x >> (64 - k))
}

fn xoshiro256pp(s: &mut PrngState) -> u64 {
    let result = rotl(s.0 + s.3, 23) + s.0;
    let t = s.1 << 17;
    s.2 ^= s.0;
    s.3 ^= s.1;
    s.1 ^= s.2;
    s.0 ^= s.3;
    s.2 ^= t;
    s.3 = rotl(s.3, 45);
    result
}

//shuffle vector
fn shuffle(v: &mut Vec<u32>, prng_state: &mut PrngState) {
    for i in 0..v.len() {
        let j = (xoshiro256pp(prng_state) as usize) % v.len();
        v.swap(i, j);
    }
}

//returns nth bit from the byte
fn get_bit(b: u8, n: u8) -> u8 {
    (b >> n) & 1
}

//extract data from image
fn extract_data(image: &RgbImage, prng_state: &mut PrngState) -> Result<Vec<u8>, String> {

    let channels = 3;
    let width = image.width();
    let hidding_spots = width * image.height() * channels ;
    let mut iidx = 0;

    //suffle vector of indices to get correct random sequence
    let mut indices: Vec<u32> = (0..hidding_spots).collect();
    shuffle(&mut indices, prng_state);

    //recover header
    let mut header: [u8; 3] = [0, 0, 0];
    for b in &mut header {
        for n in 0..8 {
            let spot_idx = indices[iidx];
            let color_offset = (spot_idx % channels) as usize;
            let pixel_idx = spot_idx / channels;
            let x = pixel_idx % width;
            let y = pixel_idx / width;

            *b |= (image.get_pixel(x, y)[color_offset] & 1) << n;
            iidx += 1;
        }
    }

    //try to recover message length and check if is possible to fit it into the image. (may not, when wrong password is used)
    let msg_len = header[0] as usize | (header[1] as usize) << 8 | (header[2] as usize) << 16;
    if msg_len * 8 + 3 > hidding_spots as usize {
        return Err("Message length from extracted header is to large to fit into this image!\nDid you use the correct key?!".to_string());
    }

    //extract message
    let mut msg = vec![0; msg_len];
    for b in &mut msg {
        for n in 0..8 {
            let spot_idx = indices[iidx];
            let color_offset = (spot_idx % channels) as usize;
            let pixel_idx = spot_idx / channels;
            let x = pixel_idx % width;
            let y = pixel_idx / width;

            *b |= (image.get_pixel(x, y)[color_offset] & 1) << n;
            iidx += 1;
        }
    }
    Ok(msg)
}

//hide data into the random pixels and random colors
fn hide_data(data: &Vec<u8>, image: &mut RgbImage, prng_state: &mut PrngState) -> Result<(), String> {

    let channels = 3;
    let width = image.width();
    let hidding_spots = width * image.height() * channels;

    if data.len() * 8 > hidding_spots as usize {
        return Err(format!("Input message is too large.\nCan't hide {} bits into {} hidding spots!", data.len() * 8, hidding_spots));
    }

    //suffle vector of indices to get random hiding spots
    let mut indices: Vec<u32> = (0..hidding_spots).collect();
    shuffle(&mut indices, prng_state);

    //hide message
    let mut iidx: usize = 0;
    for b in data {

        //hide each bit starting with LSB bit
        for n in 0..8 {
            let spot_idx = indices[iidx];
            let color_offset = (spot_idx % channels) as usize;
            let pixel_idx = spot_idx / channels;
            let x = pixel_idx % width;
            let y = pixel_idx / width;

            let pixel = image.get_pixel_mut(x, y);
            pixel[color_offset] = pixel[color_offset] & 0xfe | get_bit(*b, n);
            iidx += 1;
        }
    }
    Ok(())
}

//read file and returns content as vector
fn read_file_to_vec(path: &String) -> Result<Vec<u8>, String> {
    match File::open(path) {
        Err(s) => Err(s.to_string()),
        Ok(mut f) => {
            let mut file_data = Vec::<u8>::new();
            match f.read_to_end(&mut file_data) {
                Err(s) => Err(s.to_string()),
                Ok(_) => Ok(file_data),
            }
        }
    }
}

//write vec to the file
fn write_vec_to_file(path: &String, data: &[u8]) -> Result<usize, String> {
    match File::create(path) {
        Err(s) => Err(s.to_string()),
        Ok(mut f) => {
            match f.write(data) {
                Err(s) => Err(s.to_string()),
                Ok(n) => Ok(n),
            }
        }
    }
}

fn main() {

    //parse command line
    let mut opts = Options::new();
    opts.optflag("b","bmp", "Output image in BMP format instead of default PNG.");
    opts.optflag("x","extract", "Extract message from the image. Requires correct key.");
    opts.optopt("k", "key", "Key for embedding or extracting data.", "");
    opts.optopt("K", "key-file", "Key file for embedding or extracting data.", "");
    opts.optopt("m", "message", "Data / message to hide into the image.", "");
    opts.optopt("M", "message-file", "File with data / message to hide into the image.", "");
    opts.optflag("h","help", "Print this help and exit.");

    let matches = match opts.parse(&env::args().collect::<Vec<String>>()[1..]) {
        Ok(m) => m,
        Err(s) => { println!("{}", s); return; },
    };

    //print help end exit 
    if matches.opt_present("h") {
        println!("stegegg v{}\n\nUsage: {}", env!("CARGO_PKG_VERSION"), opts.usage("stegegg [Options] <input> <output>"));
        return;
    }

    //get key from the user or use empty one if not specified
    let user_key = if let Some(k) = matches.opt_str("k") {
        Vec::from(k.as_bytes())

    } else if let Some(file_path) = matches.opt_str("K") {
        match read_file_to_vec(&file_path){
            Ok(v) => v,
            Err(s) => { println!("{}", s); return; },
        }
    } else {
        Vec::new()
    };

    //get input file name
    let in_filename = match matches.free.first() {
        Some(f) => f,
        None => { println!("Input file not specified."); return; },
    };

    //get output file name
    let out_filename = match matches.free.get(1) {
        Some(f) => f,
        None => { println!("Output file not specified."); return; },
    };

    //open image and get the format
    let mut img = match image::io::Reader::open(in_filename) {
        Err(s) => { println!("{}", s); return; },
        Ok(r) => {
            match r.with_guessed_format() {
                Err(s) => { println!("{}", s); return; },
                Ok(r) => {
                    match r.decode() {
                        Err(s) => { println!("{}", s); return; },
                        Ok(r) => r,
                    }
                },
            }
        },
    };

    //convert it into rgb8 image
    let rgb_img = match img.as_mut_rgb8() {
        Some(r) => r,
        None => { println!("Can't convert image to rgb8!"); return; },
    };
    
    //init random generator with SHA256 frim the user key
    let mut hasher = Sha256::new();
    hasher.update(&user_key);
    let key_hash = hasher.finalize();

    //convert 32 SHA256 bytes into 4 u64.
    let mut prng_state = PrngState(
        u64::from_be_bytes(key_hash[0..8].try_into().unwrap()),
        u64::from_be_bytes(key_hash[8..16].try_into().unwrap()),
        u64::from_be_bytes(key_hash[16..24].try_into().unwrap()),
        u64::from_be_bytes(key_hash[24..32].try_into().unwrap())
    );

    //extract data from the image
    if matches.opt_present("x") {
        match extract_data(rgb_img, &mut prng_state) {
            Err(s) => println!("{}", s),
            Ok(v) => {
                match write_vec_to_file(out_filename, &v) {
                    Ok(n) => println!("{} bytes written to '{}'", n, out_filename),
                    Err(s) => println!("Error accessing the file '{}'. {}", out_filename, s),
                }
            }
        }

    //hide data into the image
    } else {

        //get message / data from the user
        let mut msg = if let Some(m) = matches.opt_str("m") {
            Vec::from(m.as_bytes())

        } else if let Some(file_path) = matches.opt_str("M") {
            match read_file_to_vec(&file_path){
                Ok(v) => v,
                Err(s) => { println!("{}", s); return; },
            }
        } else {
            println!("Input data / message not specified!\nPlease specify it with -m or -M parameter.");
            return;
        };

        //create 3 byte for message length in little endian format. This limit max message length to 16Mbytes.
        //append message to this "header"
        let mut data: Vec<u8> = vec![0, 0, 0];
        data[0] = (msg.len() & 0xff) as u8;
        data[1] = ((msg.len() >> 8) & 0xff) as u8;
        data[2] = ((msg.len() >> 16) & 0xff) as u8;
        data.append(&mut msg);

        if let Err(s) = hide_data(&data, rgb_img, &mut prng_state) {
            println!("{}", s);
            return;
        }

        //save output image
        match img.save_with_format(out_filename, if matches.opt_present("b"){ ImageFormat::Bmp } else { ImageFormat::Png }) {
            Ok(_) => println!("Message hidden in the '{}'.", out_filename),
            Err(s) => println!("Error accessing the file '{}'. {}", out_filename, s),
        }
    }
}
