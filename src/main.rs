extern crate hyper;

use hyper::Server;
use hyper::server::{Request, Response};
use hyper::uri::RequestUri;
use std::fs::File;
use std::io::prelude::{Read, Write};
use std::io::BufReader;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::env;

const USAGE: &'static str = r#"

Usage: [PORT]

Defaults to 8080.
"#;

const START: &'static str = r#"
Starting minimal webserver"#;


const OPTION: &'static str = r#"
Commandline options:
reload [ressource_name] - remove an ressource from the cache.
exit - terminate the server.
"#;

#[inline(always)]
fn default() -> Vec<u8> {
    let file = File::open("./html/index.html").expect("open failed");
    let mut file = BufReader::new(file);
    let mut buf = vec![];
    file.read_to_end(&mut buf).expect("read failed");
    buf
}

#[inline(always)]
fn unpack(uri: &RequestUri) -> String {
    match uri {
        &RequestUri::AbsolutePath(ref path) => path.clone(),
        _ => "/".to_string(),
    }
}

#[inline(always)]
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

#[inline(always)]
fn admin_input(thread_content: Arc<RwLock<HashMap<String, Vec<u8>>>>) {
    let mut line_buf = String::new();

    loop {
        line_buf.clear();
        std::io::stdout().write(b"> ").expect("stdout write");
        std::io::stdout().flush().expect("stdout flush");
        std::io::stdin().read_line(&mut line_buf).expect("stdin read");
        let line = line_buf.lines().next();

        let op = match line {
            Some(content) => {
                let mut line = content.split_whitespace();
                (line.next(), line.next())
            }
            _ => (Some(""), Some("")),
        };

        match op {
            (Some("reload"), Some(key)) => {
                match thread_content.write().unwrap().remove(key) {
                    Some(_) => println!("removed {}", key),
                    None => println!("No such asset"),
                }
            }
            (Some("exit"), _) => {
                std::process::exit(0);
            }
            _ => println!("unkown operation"),
        }
    }
}


fn main() {
    let content: Arc<RwLock<HashMap<String, Vec<u8>>>> = Arc::new(RwLock::new(HashMap::new()));
    let thread_content = content.clone();

    let host = match env::args().nth(1) {
        Some(port) => {
            println!("{} on port {}.", START, port);
            "0.0.0.0:".to_string() + &port
        }
        None => {
            println!("{} {}", START, USAGE);
            "0.0.0.0:8080".to_string()
        }
    };

    println!("{}", OPTION);

    thread::spawn(move || {
        admin_input(thread_content);
    });

    Server::http(&*host)
        .unwrap()
        .handle(move |request: Request, response: Response| {
            let key = unpack(&request.uri);
            let has_key = {
                content.read().unwrap().contains_key(&key)
            };

            let data = {
                if has_key {
                    content.read().unwrap().get(&key).unwrap().clone()
                } else {
                    let data = get_data(&key);
                    content.write().unwrap().insert(key.clone(), data.clone());
                    data
                }
            };

            response.send(data.as_slice()).unwrap();
        })
        .expect("Failed to handle client");
}
