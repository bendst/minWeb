mod args;
mod help;

extern crate hyper;
extern crate ansi_term;

use hyper::Server;
use hyper::server::{Request, Response};
use hyper::uri::RequestUri;
use hyper::status::StatusCode;
use ansi_term::Colour::{Green, Red, Blue, Black};
use std::fs::File;
use std::io::prelude::{Read, Write};
use std::io::{stdin, stdout, BufReader};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::env;
use std::thread;
use std::process::{Command, exit};

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
                println!("{}", Green.bold().paint("Shutting down"));
                exit(0);
            }
            _ => println!("{}", Red.bold().paint("unkown operation")),
        }
    }
}


fn main() {
    let mut arguments = Args::new();
    arguments.process();

    if arguments.daemon() == "--daemon" {
        Command::new(env::args().nth(0).unwrap())
            .arg("--port")
            .arg(arguments.port())
            .arg("daemon-child")
            .arg("-t")
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

        if arguments.service() != "" {

        }

        // Start server
        Server::http(&*host)
            .expect("Server creation failed")
            .handle_threads(move |request: Request, response: Response| {
                // The expected behavior after everything is cached, that only read locks will be
                // acquired which will make the server non-blocking over all threads.
                
                let key = unpack(&request.uri);

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

                response.send(data.as_slice()).expect("response send");

            }, arguments.threads())
            .expect("Failed to handle client");
    }
}
