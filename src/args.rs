use ansi_term::Colour::{Blue, Green, Red, Black};

use std::env;
use std::thread;
use std::process::{exit, Command};
use std::io::prelude::Write;
use std::io::{stdin, stdout};

use util::{USAGE, OPTION, Cache, shutting_down};
use cfi::mkfifo;

pub struct Args {
    port: String,
    daemon: String,
    threads: usize,
    service: String,
}


impl Args {
    #[inline(always)]
    pub fn new() -> Self {
        Args {
            port: "8080".to_owned(),
            daemon: "".to_owned(),
            threads: 10,
            service: "".to_owned(),
        }
    }

    #[inline(always)]
    pub fn port(&self) -> &String {
        &self.port
    }

    #[inline(always)]
    pub fn daemon(&self) -> &String {
        &self.daemon
    }

    #[inline(always)]
    pub fn threads(&self) -> usize {
        self.threads
    }

    #[inline(always)]
    pub fn service(&self) -> &String {
        &self.service
    }

    #[inline(always)]
    pub fn has_service(&self) -> bool {
        &self.service != ""
    }

    /// Process passed commandline arguments and set Args appropriate
    #[inline(always)]
    pub fn process(&mut self) {
        for arg in env::args().enumerate() {
            match (arg.0, &arg.1 as &str) {
                (pos, "--port") => {
                    self.port = env::args().nth(pos + 1).expect("No port");
                }
                (pos, "-p") => {
                    self.port = env::args().nth(pos + 1).expect("No port");
                }
                (_, e @ "--daemon") => {
                    self.daemon = e.to_owned();
                }
                (_, e @ "daemon-child") => {
                    self.daemon = e.to_owned();
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
                (pos, "--service") => {
                    self.service = env::args().nth(pos + 1).expect("mssing service")
                }
                _ => (),
            }
        }
    }

    #[inline(always)]
    pub fn make_service(&self) {
        if self.service != "" {
            let path = env::temp_dir().join("http_service_in.pipe");
            mkfifo(path.to_str().unwrap());

            let path = env::temp_dir().join("http_service_out.pipe");
            mkfifo(path.to_str().unwrap());
            Command::new(&self.service).spawn().unwrap();
        }
    }


    #[inline(always)]
    pub fn make_daemon(&self, cache: Cache) {
        if self.daemon != "daemon-child" {
            println!("{}", Blue.paint(OPTION));
            thread::spawn(move || {
                Args::admin_input(cache);
            });
        }
    }

    /// handles admin input after a change to the html, css or js files.
    /// It is possible to remove items from the cache or shutdown the server.
    #[inline(always)]
    pub fn admin_input(thread_content: Cache) {
        let mut line_buf = String::new();

        loop {
            line_buf.clear();
            stdout().write(b"> ").expect("stdout write");
            stdout().flush().expect("stdout flush");
            stdin().read_line(&mut line_buf).expect("stdin read");
            let line = line_buf.lines().next();

            let op = line.map_or((Some(""), Some("")), |content| {
                let mut line = content.split_whitespace();
                (line.next(), line.next())
            });

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
                    shutting_down(0);
                }
                _ => println!("{}", Red.bold().paint("unkown operation")),
            }
        }
    }
}
