extern crate hyper;
extern crate ansi_term;

use hyper::Server;
use hyper::server::{Request, Response};
use hyper::uri::RequestUri;
use hyper::status::StatusCode;
use ansi_term::Colour::{Green, Red, Blue, Black};
use std::fs::File;
use std::io::prelude::{Read, Write};
use std::io::BufReader;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::env;
use std::process::{Command, exit};

const USAGE: &'static str = r#"
Usage: [-p | --port PORT] [--daemon] [--help | -h] [-t MAX_THREADS]
Defaults to 8080.
"#;

const START: &'static str = r#"
Starting minimal webserver"#;


const OPTION: &'static str = r#"
Commandline options:
reload [NAME] - remove an ressource from the cache.
reload * - remove all ressources from the cache.
reload all - remove all ressources from the cache.
reload - remove all ressources from the cache.
exit - terminate the server.
"#;

/// Default size of vector.
const SIZE: usize = 4 * 1024;

/// Convenience type for a thread safe Hashmap.
type Cache = Arc<RwLock<HashMap<String, Vec<u8>>>>;


macro_rules! read_from_file {
    ($path: expr) => (
        {
        let file = File::open($path).expect("open failed ");
        let mut file = BufReader::new(file);
        let mut buf = Vec::with_capacity(SIZE);
        file.read_to_end(&mut buf).expect("read failed");
        buf
        }
    );
}


/// Reads the index.html as default action.
#[inline(always)]
fn default() -> Vec<u8> {
    read_from_file!("./html/index.html")
}


/// Unpack an Uri to a String.
#[inline(always)]
fn unpack(uri: &RequestUri) -> String {
    match uri {
        &RequestUri::AbsolutePath(ref path) => path.clone(),
        _ => "/".to_string(),
    }
}


/// Retrieve data from fs instead of the cache.
/// In case of an error the index.html is returned.
#[inline(always)]
fn get_data(path: &String) -> Vec<u8> {
    let mut data = String::from(".");
    if path != "/" {
        data.push_str(&path);
        let file = File::open(data);
        match file {
            Ok(file) => {
                let mut file = BufReader::new(file);
                let mut buf = Vec::with_capacity(SIZE);
                file.read_to_end(&mut buf).expect("read failed");
                buf
            }
            Err(_) => StatusCode::NotFound.to_string().into(),
        }
    } else {
        default()
    }
}


/// handles admin input after a change to the html, css or js files.
/// It is possible to remove items from the cache or shutdown the server.
#[inline(always)]
fn admin_input(thread_content: Cache) {
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


/// Structure holding passed arguments
struct Args {
    port: String,
    daemon: String,
    threads: usize,
}


impl Args {
    #[inline(always)]
    fn new() -> Self {
        Args {
            port: "8080".to_string(),
            daemon: "".to_string(),
            threads: 10,
        }
    }


    /// Process passed commandline arguments and set Args appropriate
    #[inline(always)]
    fn process(&mut self) {
        for arg in env::args().enumerate() {
            match (arg.0, &arg.1 as &str) {
                (pos, "--port") => {
                    self.port = env::args().nth(pos + 1).expect("No port");
                }
                (pos, "-p") => {
                    self.port = env::args().nth(pos + 1).expect("No port");
                }
                (_, e @ "--daemon") => {
                    self.daemon = e.to_string();
                }
                (_, e @ "daemon-child") => {
                    self.daemon = e.to_string();
                }
                (pos, "-t") => {
                    self.threads = env::args()
                                       .nth(pos + 1)
                                       .expect("missing thread count")
                                       .parse()
                                       .unwrap();
                }
                (_, "--help") => {
                    println!("{} {}", Blue.paint(USAGE), Blue.paint(OPTION));
                    exit(0);
                }
                (_, "-h") => {
                    println!("{} {}", Blue.paint(USAGE), Blue.paint(OPTION));
                    exit(0);
                }
                _ => (),
            }
        }
    }
}


fn main() {
    let mut arguments = Args::new();
    arguments.process();

    if arguments.daemon == "--daemon" {
        Command::new(env::args().nth(0).unwrap())
            .arg("--port")
            .arg(arguments.port)
            .arg("daemon-child")
            .arg("-t")
            .arg(arguments.threads.to_string())
            .spawn()
            .expect("Daemon could not be summoned");
    } else {

        let content: Cache = Arc::new(RwLock::new(HashMap::new()));
        let thread_content = content.clone();

        // check whether a port was specified.
        let host = match &*arguments.port {
            "" => {
                println!("{} {}", Green.bold().paint(START), Blue.paint(USAGE));
                "0.0.0.0:8080".to_string()
            }
            port => {
                println!("{} {} {}.",
                         Green.bold().paint(START),
                         Green.bold().paint("on port"),
                         Red.bold().paint(port.clone()));
                "0.0.0.0:".to_string() + &port
            }
        };

        if arguments.daemon != "daemon-child" {
            // Spawn the thread for admin input.
            println!("{}", Blue.paint(OPTION));
            thread::spawn(move || {
                admin_input(thread_content);
            });
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
                        content.write().expect("write lock").insert(key.clone(), data.clone());
                        data
                    }
                }; // release read or write lock dependent on has_key.

                response.send(data.as_slice()).expect("response send");

            }, arguments.threads)
            .expect("Failed to handle client");
    }
}
