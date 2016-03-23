extern crate hyper;
extern crate ansi_term;
extern crate libc;

mod args;
mod util;
mod cfi;

use hyper::Server;
use hyper::server::{Request, Response};
use hyper::uri::RequestUri;
use hyper::status::StatusCode;
use ansi_term::Colour::{Green, Red, Blue};

use std::env;
use std::thread;
use std::process::Command;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc::channel;

use args::Args;
use util::{START, USAGE, Cache};

/// Default size of vector.
const SIZE: usize = 4 * 1024;

macro_rules! read_from_file {
    ($file: expr) => (
        {
        let mut file = BufReader::new($file);
        let mut buf = Vec::with_capacity(SIZE);
        file.read_to_end(&mut buf).expect("read failed");
        buf
        }
    );
}


/// Reads the index.html as default action.
#[inline(always)]
fn default() -> Vec<u8> {
    let file = File::open("./html/index.html").expect("open failed");
    read_from_file!(file)
}


/// Unpack an Uri to a String.
#[inline(always)]
fn unpack(uri: &RequestUri) -> String {
    match uri {
        &RequestUri::AbsolutePath(ref path) => path.clone(),
        _ => "/".to_owned(),
    }
}


/// Retrieve data from fs instead of the cache.
/// In case of an error the index.html is returned.
#[inline(always)]
fn get_data(path: &String) -> Option<Vec<u8>> {
    let mut data = String::from(".");
    if path != "/" {
        data.push_str(&path);
        let file = File::open(data);
        match file {
            Ok(file) => Some(read_from_file!(file)),
            Err(_) => None,
        }
    } else {
        Some(default())
    }
}

fn process_io(receiver: Arc<Mutex<Receiver<Vec<u8>>>>, sender: Arc<Mutex<Sender<Vec<u8>>>>) {
    let sender = sender.clone();
    thread::spawn(move || {
        let in_fd = File::open("/tmp/http_service_in.pipe").expect("could not open fifo in");
        let mut in_fd = BufWriter::new(in_fd);

        let out_fd = File::open("/tmp/http_service_out.pipe").expect("could not open fifo out");
        let mut out_fd = BufReader::new(out_fd);

        loop {
            let incoming = receiver.lock().unwrap().recv().unwrap();
            in_fd.write_all(incoming.clone().as_slice()).expect("write failed");

            let mut out_data = vec![];
            out_fd.read_to_end(&mut out_data).expect("read failed");
            sender.lock().unwrap().send(out_data).unwrap();
        }
    });
}



pub fn main() {
    let mut arguments = Args::new();
    arguments.process();

    cfi::sigaction(libc::SIGINT, util::shutting_down);
    cfi::sigaction(libc::SIGTERM, util::shutting_down);

    if arguments.daemon() == "--daemon" {
        Command::new(env::args().nth(0).unwrap())
            .arg("--port")
            .arg(arguments.port())
            .arg("daemon-child")
            .arg("-t")
            .arg(arguments.threads().to_string())
            .arg("--service")
            .arg(arguments.service())
            .spawn()
            .expect("Daemon could not be summoned");
    } else {

        let content: Cache = Arc::new(RwLock::new(HashMap::new()));
        let thread_content = content.clone();


        // check whether a port was specified.
        let host = match &**arguments.port() {
            "" => {
                println!("{} {}", Green.bold().paint(START), Blue.paint(USAGE));
                "0.0.0.0:8080".to_owned()
            }
            port => {
                println!("{} {} {}.",
                         Green.bold().paint(START),
                         Green.bold().paint("on port"),
                         Red.bold().paint(port.clone()));
                "0.0.0.0:".to_owned() + &port
            }
        };

        arguments.make_daemon(thread_content);
        arguments.make_service();


        let (sender_x, receiver_x) = channel();
        let (sender_y, receiver_y) = channel();

        let sender_x = Arc::new(Mutex::new(sender_x));
        let sender_y = Arc::new(Mutex::new(sender_y));
        let receiver_x = Arc::new(Mutex::new(receiver_x));
        let receiver_y = Arc::new(Mutex::new(receiver_y));

        process_io(receiver_x, sender_y);


        // Start server
        Server::http(&*host)
            .expect("Server creation failed")
            .handle_threads(move |mut request: Request, response: Response| {
                // The expected behavior after everything is cached, that only read locks will be
                // acquired which will make the server non-blocking over all threads.
                
                let key = unpack(&request.uri);
                
                let data = if key.contains("service") {
                    let mut service_data = vec![];
                    request.read_to_end(&mut service_data).expect("read failed");
                    sender_x.lock().unwrap().send(service_data).unwrap();
                    receiver_y.lock().unwrap().recv().unwrap()
                } else {
                    let has_key = {
                        content.read().expect("read lock").contains_key(&key)
                    }; // release read lock.

                    let data = match has_key {
                        true => content.read().expect("read lock").get(&key).unwrap().clone(),
                        _ => {
                            let data = get_data(&key);
                            match data {
                                Some(data) => {
                                    content.write().expect("write lock").insert(key.clone(), data.clone());
                                    data
                                },
                                None => StatusCode::NotFound.canonical_reason().unwrap().to_owned().into(),
                            }
                        }
                    }; // release read or write lock dependent on has_key.
                    data
                };

                response.send(data.as_slice()).expect("response send");

            }, arguments.threads())
            .expect("Failed to handle client");
    }
}
