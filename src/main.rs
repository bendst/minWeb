extern crate hyper;

use hyper::Server;
use hyper::server::{Request, Response};
use hyper::uri::RequestUri;
use std::fs::File;
use std::io::prelude::Read;
use std::io::BufReader;
use std::collections::HashMap;
use std::sync::Mutex;
use std::env;

fn default() -> Vec<u8> {
    let file = File::open("./html/index.html").expect("open failed");
    let mut file = BufReader::new(file);
    let mut buf = vec![];
    file.read_to_end(&mut buf).expect("read failed");
    buf
}

fn unpack(uri: &RequestUri) -> String {
    match uri {
        &RequestUri::AbsolutePath(ref path) => path.clone(),
        _ => "/".to_string(),
    }
}

fn get_data(path: &String) -> Vec<u8> {
    let mut data = String::from(".");
    if path != "/" {
        data.push_str(&path);
        let file = File::open(data);
        match file {
            Ok(file) => {
                let mut file = BufReader::new(file);
                let mut buf = vec![];
                file.read_to_end(&mut buf).expect("read failed");
                buf
            }
            Err(_) => default(),
        }
    } else {
        default()
    }
}

fn main() {
    let content: Mutex<HashMap<String, Vec<u8>>> = Mutex::new(HashMap::new());

    let host = match env::args().nth(1) {
        Some(port) => "127.0.0.1:".to_string() + &port,
        None => "127.0.0.1:8080".to_string(),
    };

    Server::http(&*host)
        .unwrap()
        .handle(move |request: Request, response: Response| {
            let key = unpack(&request.uri);

            let has_key = {
                let content = content.lock().unwrap();
                content.contains_key(&key)
            };

            let data = {
                if has_key {
                    content.lock().unwrap().get(&key).unwrap().clone()
                } else {
                    let data = get_data(&key);
                    content.lock().unwrap().insert(key.clone(), data.clone());
                    data
                }
            };

            response.send(data.as_slice()).unwrap();
        })
        .expect("Failed to handle client");
}
