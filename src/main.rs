use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    os::unix,
    path::Path,
    process::Command,
};

mod arguments_parser;
mod config;
mod config_updaters;
mod json_file_updater;

use rand::{distributions::Alphanumeric, Rng};

use chrono::prelude::Utc;

struct ArgumentDescription {
    name: &'static str,
    syntax: &'static str,
    description: &'static str,
}

const ARGUMENTS: [ArgumentDescription; 3] = [
    ArgumentDescription {
        name: "help",
        syntax: "help",
        description: "Print this help message",
    },
    ArgumentDescription {
        name: "config",
        syntax: "config <path>",
        description: "Set path to the config file",
    },
    ArgumentDescription {
        name: "generate-default-config",
        syntax: "generate-default-config",
        description: "Generate default config file",
    },
];

fn main() {
    let arguments = arguments_parser::ArgumentsParser::new(std::env::args().collect());

    let mut has_unknown_arguments = false;
    arguments.for_each_argument(|argument| {
        if !ARGUMENTS.iter().any(|arg| arg.name == argument.name) {
            println!("Unknown argument: {}", argument.name);
            has_unknown_arguments = true;
        }
    });

    if has_unknown_arguments {
        return;
    }

    if arguments.has_argument("help") {
        println!("Usage: matchmaker [arguments]");
        println!("Arguments:");
        for argument in &ARGUMENTS {
            println!("  --{}: {}", argument.syntax, argument.description);
        }
        return;
    }

    if arguments.has_argument("generate-default-config") {
        config::generate_default_config("data/config.json");
        return;
    }

    let config_path = arguments.get_value("config").unwrap_or("data/config.json".to_string());

    let config = config::read_config(&config_path);
    let config = match config {
        Ok(config) => config,
        Err(error) => {
            println!("Error reading config: {}.\nUse --generate-default-config to generate default config", error);
            return;
        }
    };

    // create the directory for the working directories
    fs::create_dir_all(&config.working_directiries_path).unwrap_or_else(|error| {
        println!(
            "Problem creating directory '{}': {:?}",
            config.working_directiries_path, error
        );
    });

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.matchmaker_port)).unwrap();
    let interface = listener.local_addr().unwrap_or_else(|error| {
        panic!("Problem getting local address: {:?}", error);
    }).ip().to_string();

    println!(
        "Matchmaker service started on inteface {} port {}",
        interface,
        config.matchmaker_port
    );

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(
            stream,
            &config.working_directiries_path,
            &config.dedicated_server_dir,
            &interface,
        );
    }
}

fn start_dedicated_server(
    port: u16,
    dedicated_server_working_dir: &str,
    dedicated_server_dir: &str,
) -> Result<std::process::Child, std::io::Error> {
    return Command::new("./run_detached_process.sh")
        .arg(dedicated_server_working_dir)
        .arg(format!("{}/DedicatedServer", dedicated_server_dir))
        .arg(format!("--open-port {}", port))
        .spawn();
}

fn generate_unique_directory(working_dir: &str) -> String {
    let random_string_part: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect();

    // format: YYMMDD_HHMMSS_rand
    let mut unique_name = Utc::now().format("%y%m%d_%H%M%S_").to_string() + &random_string_part;
    while Path::new(working_dir).join(&unique_name).is_dir() {
        let random_string_part: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        unique_name = Utc::now().format("%y%M%d_%H%m%s_").to_string() + &random_string_part;
    }

    return Path::new(working_dir)
        .join(&unique_name)
        .to_string_lossy()
        .to_string();
}

fn create_dedicated_server_environment(
    dedicated_server_working_dir: &str,
    dedicated_server_dir: &str,
) {
    fs::create_dir_all(dedicated_server_working_dir).unwrap_or_else(|error| {
        println!(
            "Problem creating directory '{}': {:?}",
            dedicated_server_working_dir, error
        );
    });
    unix::fs::symlink(
        format!("{}/resources", dedicated_server_dir),
        Path::new(dedicated_server_working_dir).join("resources"),
    )
    .unwrap();
}

fn process_one_line_request(
    http_request: Vec<String>,
    working_directories_path: &str,
    dedicated_server_dir: &str,
    interface: &str,
) -> Option<String> {
    if http_request[0] == "connect" {
        let port: Option<u16> = get_available_port(interface);
        match port {
            Some(val) => {
                let new_server_working_dir = generate_unique_directory(working_directories_path);
                create_dedicated_server_environment(&new_server_working_dir, dedicated_server_dir);
                match start_dedicated_server(val, &new_server_working_dir, dedicated_server_dir) {
                    Ok(_) => {
                        println!("Spawned new dedicated server on port {}", val);
                        return Some(val.to_string())
                    },
                    Err(error) => return Some(format!("Problem opening the file: {:?}", error)),
                }
            }
            None => return Some("no ports".to_string()),
        }
    } else {
        println!("Unknown one line request: {:#?}", http_request);
        return None;
    }
}

fn handle_connection(
    mut stream: TcpStream,
    working_directories_path: &str,
    dedicated_server_dir: &str,
    interface: &str,
) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    if http_request.len() == 1 {
        let response =
            process_one_line_request(http_request, working_directories_path, dedicated_server_dir, interface);

        match response {
            Some(val) => stream.write_all(val.as_bytes()).unwrap(),
            None => {}
        }
    } else {
        println!("Unknown request: {:#?}", http_request);
    }
}

fn get_available_port(interface: &str) -> Option<u16> {
    (8000..9000).find(|port| is_port_available(interface, *port))
}

fn is_port_available(interface: &str, port: u16) -> bool {
    match TcpListener::bind((interface, port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}
