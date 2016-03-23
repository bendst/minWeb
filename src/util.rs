use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::env;
use ansi_term::Colour::Green;
use cfi;
use std::process::exit;
/// Convenience type for a thread safe Hashmap.
pub type Cache = Arc<RwLock<HashMap<String, Vec<u8>>>>;



pub const USAGE: &'static str = r#"
Usage: [-p | --port PORT] [--daemon] [--help | -h] [-t MAX_THREADS]
Defaults to 8080.
"#;

pub const START: &'static str = r#"
Starting minimal webserver"#;


pub const OPTION: &'static str = r#"
Commandline options:
reload [NAME] - remove an ressource from the cache.
reload * - remove all ressources from the cache.
reload all - remove all ressources from the cache.
reload - remove all ressources from the cache.
exit - terminate the server.
"#;

pub fn shutting_down(_: i32) {
    let path = env::temp_dir().join("http_service_in.pipe");
    cfi::unlink(path.to_str().unwrap());

    let path = env::temp_dir().join("http_service_out.pipe");
    cfi::unlink(path.to_str().unwrap());

    println!("{}", Green.bold().paint("Shutting down"));
    exit(0);
}
