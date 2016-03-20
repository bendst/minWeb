
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
