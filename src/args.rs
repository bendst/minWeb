use std::env;
use ansi_term::Colour::Blue;
use help::{USAGE, OPTION};
use std::process::exit;

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
    pub fn port(&self) -> &String {
        &self.port
    }

    pub fn daemon(&self) -> &String {
        &self.daemon
    }

    pub fn threads(&self) -> usize {
        self.threads
    }

    pub fn service(&self) -> &String {
        &self.service
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
}
