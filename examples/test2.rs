extern crate bincode;
extern crate rustc_serialize;

use std::fs::File;
//use rustc_serialize::bincode;
use bincode::rustc_serialize::{encode, decode};

#[derive(RustcEncodable, RustcDecodable)]
struct A {
  id: i8,
  key: i16,
  name: String,
  values: Vec<String>
}

fn main() {
    let a = A {
        id: 42,
        key: 1337,
        name: "Hello world".to_string(),
        values: vec!["alpha".to_string(), "beta".to_string()],
    };

    // Encode to something implementing Write
    //let mut f = File::create("/tmp/output.bin").unwrap();
    //bincode::encode_into(&a, &mut f, bincode::SizeLimit::Infinite).unwrap();

    // Or just to a buffer
    let bytes = encode(&a, bincode::SizeLimit::Infinite).unwrap();
    let decoded: A = decode(&bytes[..]).unwrap();

    /*
    let their_info = match json::decode(&bytes) {
        Ok(info) => info,
        Err(e) => {
            println!("Error decoding their connection info");
            println!("{}", e);
        },
    };*/
    //println!("bytes: {:?}", bytes);
    //println!("A: {}", decoded.id);
    //println!("their_info {:?}", their_info);
    let msg = "1 2 3 4 5 6";
    let msgs: Vec<_> = msg.trim_right_matches(|c| c == '\r' || c == '\n')
        .split(' ')
        .collect::<Vec<_>>().drain(2..).collect();

    println!("{}", msgs.into_string());

    //let mut m: Vec<_> = msgs.drain(2..).collect();
    //println!("{:?}", m);
}
