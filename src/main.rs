use std::net::UdpSocket;
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
use crate::config::Config;

const MATCHMAKER_PROTOCOL_VERSION: &str = "1";

struct MatchmakerState {
    players_waiting: Vec<String>,
}

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

    let config_path = arguments
        .get_value("config")
        .unwrap_or("data/config.json".to_string());

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

    if !validate_dedicated_server_executable_path(&config) { return; }

    let listener = TcpListener::bind(format!(
        "{}:{}",
        config.network_interface, config.matchmaker_port
    ))
    .unwrap();
    let interface = listener
        .local_addr()
        .unwrap_or_else(|error| {
            panic!("Problem getting local address: {:?}", error);
        })
        .ip()
        .to_string();

    println!(
        "Matchmaker service started on inteface {} port {}",
        interface, config.matchmaker_port
    );

    let state = std::sync::Arc::new(std::sync::Mutex::new(MatchmakerState {
        players_waiting: Vec::new(),
    }));

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(
            stream,
            &config.working_directiries_path,
            &config.dedicated_server_dir,
            &interface,
            state.clone(),
        );
    }
}

fn validate_dedicated_server_executable_path(config: &Config) -> bool {
    if Path::new(&config.dedicated_server_dir).is_absolute() {
        let path = Path::new(&config.dedicated_server_dir).join("DedicatedServer");
        if !path.is_file() {
            println!(
                "Dedicated server executable '{}' can't be found",
                path.to_string_lossy()
            );
            return false;
        }
    } else {
        let path = Path::new(&config.working_directiries_path)
            .join(Path::new(&config.dedicated_server_dir).strip_prefix("../").unwrap())
            .join("DedicatedServer");
        if !path.is_file() {
            println!(
                "Dedicated server executable '{}' can't be found",
                path.to_string_lossy()
            );
            return false;
        }
    }
    true
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

fn start_new_server(
    working_directories_path: &str,
    dedicated_server_dir: &str,
    interface: &str,
) -> Option<String> {
    let port: Option<u16> = get_available_port(interface);
    match port {
        Some(val) => {
            let new_server_working_dir = generate_unique_directory(working_directories_path);
            create_dedicated_server_environment(&new_server_working_dir, dedicated_server_dir);
            match start_dedicated_server(val, &new_server_working_dir, dedicated_server_dir) {
                Ok(_) => {
                    println!("Spawned new dedicated server on port {}", val);
                    return Some(val.to_string());
                }
                Err(error) => return Some(format!("Problem opening the file: {:?}", error)),
            }
        }
        None => return Some("no ports".to_string()),
    }
}

fn process_one_line_request(
    request: &String,
    working_directories_path: &str,
    dedicated_server_dir: &str,
    interface: &str,
    state: std::sync::Arc<std::sync::Mutex<MatchmakerState>>,
) -> Option<String> {
    if request == "protocol-version" {
        return Some(MATCHMAKER_PROTOCOL_VERSION.to_string());
    }
    if request == "connect" {
        let mut state = state.lock().unwrap();
        if state.players_waiting.len() > 0 {
            let port = state.players_waiting.pop().unwrap();
            return Some(format!("port:{}", &port));
        } else {
            state.players_waiting.push(
                start_new_server(working_directories_path, dedicated_server_dir, interface)
                    .unwrap(),
            );
            return Some(format!("port:{}", &state.players_waiting[0]));
        }
    } else {
        println!("Unknown one line request: {:#?}", request);
        return None;
    }
}

fn handle_connection(
    mut stream: TcpStream,
    working_directories_path: &str,
    dedicated_server_dir: &str,
    interface: &str,
    state: std::sync::Arc<std::sync::Mutex<MatchmakerState>>,
) {
    while let Ok(_) = stream.set_read_timeout(Some(std::time::Duration::from_millis(100))) {
        let mut reader = BufReader::new(&stream);
        let mut http_request = String::new();
        let read_result = reader.read_line(&mut http_request);
        if read_result.is_err() {
            break;
        }
        let read_result = read_result.unwrap();
        if read_result == 0 {
            break;
        }
        let http_request: Vec<String> = http_request
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if http_request.len() == 1 {
            let response = process_one_line_request(
                &http_request[0],
                working_directories_path,
                dedicated_server_dir,
                interface,
                state.clone(),
            );

            match response {
                Some(val) => {
                    println!("Responding with: {}", val);
                    stream.write_all(val.as_bytes()).unwrap()
                }
                None => {}
            }
        } else {
            println!("Unknown request: {:#?}", http_request);
        }
    }
}

fn get_available_port(interface: &str) -> Option<u16> {
    (8000..9000).find(|port| is_port_available(interface, *port))
}

fn is_port_available(interface: &str, port: u16) -> bool {
    match UdpSocket::bind((interface, port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}
