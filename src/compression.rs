use flate2::write::DeflateEncoder;
use flate2::bufread::DeflateDecoder;

use flate2::Compression;
use std::io::prelude::*;
use std::time::{Instant};

fn create_data(width: usize, height: usize) -> Vec<u8> {
    let size = width * height * 3;
    let mut data = Vec::with_capacity(size);
    for i in 0..size {
        data.push((i * i + i) as u8);
    }
    data
}

fn main() {
    let data = create_data(1920, 1080);
    println!("Data before is {} KB", data.len() / 1024);

    let start = Instant::now();
    let compressed = encode(&data);
    println!("Data after is {} KB", compressed.len() / 1024);
    println!("Compression took {} millis", start.elapsed().as_millis());

    let start = Instant::now();
    let uncompressed = decode(&compressed);
    println!("Data back is {} KB", uncompressed.len() / 1024);
    println!("Decompression took {} millis", start.elapsed().as_millis());
}

fn encode(raw: &Vec<u8>) -> Vec<u8> {
    let mut e = DeflateEncoder::new(Vec::new(), Compression::best());
    e.write_all(&raw).unwrap();
    e.finish().unwrap()
}

fn decode(compressed: &Vec<u8>) -> Vec<u8> {
    let mut result = Vec::new();
    let mut deflater = DeflateDecoder::new(&compressed[..]);
    deflater.read_to_end(&mut result).unwrap();
    result
}
