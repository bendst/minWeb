mod args;
mod help;

extern crate hyper;
extern crate ansi_term;
extern crate libc;

use hyper::Server;
use hyper::server::{Request, Response};
use hyper::uri::RequestUri;
use hyper::status::StatusCode;
use ansi_term::Colour::{Green, Red, Blue, Black};
use std::fs::File;
use std::io::prelude::*;
use std::io::{stdin, stdout, BufReader, BufWriter};
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc::channel;
use std::env;
use std::ffi::CString;
use std::thread;
use std::process::{Command, exit, Stdio};
use libc::{mkfifo, unlink};
use args::Args;
use help::{START, USAGE, OPTION};

/// Default size of vector.
const SIZE: usize = 4 * 1024;

/// Convenience type for a thread safe Hashmap.
type Cache = Arc<RwLock<HashMap<String, Vec<u8>>>>;


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


/// handles admin input after a change to the html, css or js files.
/// It is possible to remove items from the cache or shutdown the server.
#[inline(always)]
fn admin_input(thread_content: Cache) {
    let mut line_buf = String::new();

    loop {
        line_buf.clear();
        stdout().write(b"> ").expect("stdout write");
        stdout().flush().expect("stdout flush");
        stdin().read_line(&mut line_buf).expect("stdin read");
        let line = line_buf.lines().next();

        let op = match line {
            Some(content) => {
                let mut line = content.split_whitespace();
                (line.next(), line.next())
            }
            _ => (Some(""), Some("")),
        };

        match op {
            (Some("reload"), Some("*")) => {
                thread_content.write().unwrap().clear();
                println!("{}", Green.bold().paint("Cache cleared"));
            }
            (Some("reload"), Some("all")) => {
                thread_content.write().unwrap().clear();
                println!("{}", Green.bold().paint("Cache cleared"));
            }
            (Some("reload"), None) => {
                thread_content.write().unwrap().clear();
                println!("{}", Green.bold().paint("Cache cleared"));
            }

            (Some("reload"), Some(key)) => {
                match thread_content.write().unwrap().remove(key) {
                    Some(_) => {
                        println!("{} {}",
                                 Green.bold().paint("removed"),
                                 Black.bold().paint(key))
                    }
                    None => println!("{}", Red.bold().paint("No such asset")),
                }
            }
            (Some("exit"), _) => {
                unsafe {
                    unlink(CString::new("/tmp/http_service_in.pipe").unwrap().as_ptr());
                    unlink(CString::new("/tmp/http_service_out.pipe").unwrap().as_ptr());
                }
                println!("{}", Green.bold().paint("Shutting down"));
                exit(0);
            }
            _ => println!("{}", Red.bold().paint("unkown operation")),
        }
    }
}


fn shutting_down(signum: i32) {
    unsafe {
        unlink(CString::new("/tmp/http_service_in.pipe").unwrap().as_ptr());
        unlink(CString::new("/tmp/http_service_out.pipe").unwrap().as_ptr());
    }
    println!("{}", Green.bold().paint("Shutting down"));
    exit(0);
}

pub fn main() {
    let mut arguments = Args::new();
    arguments.process();

    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = shutting_down as usize;
        libc::sigaction(libc::SIGINT, &action, std::mem::zeroed());
    }

    if arguments.daemon() == "--daemon" {
        Command::new(env::args().nth(0).unwrap())
            .arg("--port")
            .arg(arguments.port())
            .arg("daemon-child")
            .arg("-t")
            .arg("--service")
            .arg(arguments.service())
            .arg(arguments.threads().to_string())
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

        if arguments.daemon() != "daemon-child" {
            // Spawn the thread for admin input.
            println!("{}", Blue.paint(OPTION));
            thread::spawn(move || {
                admin_input(thread_content);
            });
        }
        let service = if arguments.service() != "" {
            unsafe {
                mkfifo(CString::new("/tmp/http_service_in.pipe").unwrap().as_ptr(),
                       libc::S_IRUSR | libc::S_IWUSR);
                mkfifo(CString::new("/tmp/http_service_out.pipe").unwrap().as_ptr(),
                       libc::S_IRUSR | libc::S_IWUSR);
            }
            Command::new(arguments.service())
                .spawn()
                .unwrap()
        } else {
            Command::new("").spawn().unwrap()
        };

        let x: Arc<Mutex<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>> = Arc::new(Mutex::new(channel()));
        let x2: Arc<Mutex<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>> = Arc::new(Mutex::new(channel()));
        let x_1 = x.clone();
        let x_2 = x2.clone();

        thread::spawn(move || {
            let in_fd = File::open("/tmp/http_service_in.pipe").expect("could not open fifo in");
            let mut in_fd = BufWriter::new(in_fd);

            let out_fd = File::open("/tmp/http_service_out.pipe").expect("could not open fifo out");
            let mut out_fd = BufReader::new(out_fd);

            loop {
                let incoming = x_1.lock().unwrap().1.recv().unwrap();
                in_fd.write_all(incoming.as_slice());

                let mut out_data = vec![];
                out_fd.read_to_end(&mut out_data);
                x_2.lock().unwrap().0.send(out_data).unwrap();
            }
        });

        let x_11 = x.clone();
        let x_22 = x2.clone();

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
                    x_11.lock().unwrap().0.send(service_data).unwrap();
                    x_22.lock().unwrap().1.recv().unwrap()
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
