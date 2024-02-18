use std::{
    env, fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    os::unix,
    path::Path,
    process::Command,
};

mod config_updaters;
mod json_file_updater;
mod config;

use rand::{distributions::Alphanumeric, Rng};

use chrono::prelude::Utc;

fn main() {
    if env::args().len() == 2 && env::args().nth(1).unwrap() == "--generate-default-config" {
        config::generate_default_config();
        return;
    }

    let config = config::read_config();
    let config = match config {
        Ok(config) => config,
        Err(error) => {
            panic!("Error reading config: {}.\nUse --generate-default-config to generate default config", error);
            return;
        }
    };

    // create the directory for the working directories
    fs::create_dir_all(&config.working_directiries_path).unwrap_or_else(|error| {
        println!("Problem creating directory '{}': {:?}", config.working_directiries_path, error);
    });

    let listener = TcpListener::bind(format!("127.0.0.1:{}", config.matchmaker_port)).unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream, &config.working_directiries_path, &config.dedicated_server_dir);
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
    dedicated_server_dir: &str) {
    fs::create_dir_all(dedicated_server_working_dir).unwrap_or_else(|error| {
        println!("Problem creating directory '{}': {:?}", dedicated_server_working_dir, error);
    });
    unix::fs::symlink(format!("{}/resources", dedicated_server_dir), Path::new(dedicated_server_working_dir).join("resources")).unwrap();
}

fn process_one_line_request(http_request: Vec<String>, working_directories_path: &str, dedicated_server_dir: &str) -> Option<String> {
    if http_request[0] == "connect" {
        let port: Option<u16> = get_available_port();
        match port {
            Some(val) => {
                let new_server_working_dir = generate_unique_directory(working_directories_path);
                create_dedicated_server_environment(&new_server_working_dir, dedicated_server_dir);
                match start_dedicated_server(val, &new_server_working_dir, dedicated_server_dir) {
                    Ok(_) => return Some(val.to_string()),
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

fn handle_connection(mut stream: TcpStream, working_directories_path: &str, dedicated_server_dir: &str) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    if http_request.len() == 1 {
        let response = process_one_line_request(http_request, working_directories_path, dedicated_server_dir);

        match response {
            Some(val) => stream.write_all(val.as_bytes()).unwrap(),
            None => {}
        }
    } else {
        println!("Unknown request: {:#?}", http_request);
    }
}

fn get_available_port() -> Option<u16> {
    (8000..9000).find(|port| is_port_available(*port))
}

fn is_port_available(port: u16) -> bool {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}
