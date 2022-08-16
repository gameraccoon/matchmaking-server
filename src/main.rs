use std::{
    env, fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    os::unix,
    path::Path,
    process::Command,
};

use rand::{distributions::Alphanumeric, Rng};

use chrono::prelude::Utc;

fn main() {
    let args: Vec<String> = env::args().collect();
    let working_dir = if args.len() == 2 { &args[1] } else { "." };
    let listener = TcpListener::bind("127.0.0.1:14736").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream, working_dir);
    }
}

fn start_dedicated_server(
    port: u16,
    working_dir: &str,
) -> Result<std::process::Child, std::io::Error> {
    return Command::new("./run_detached_process.sh")
        .arg(working_dir)
        .arg("../DedicatedServer")
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

fn create_dedicated_server_environment(working_dir: &str) {
    fs::create_dir(working_dir).unwrap();
    unix::fs::symlink("../../resources", Path::new(working_dir).join("resources")).unwrap();
}

fn process_one_line_request(http_request: Vec<String>, working_dir: &str) -> Option<String> {
    if http_request[0] == "start-server" {
        let port: Option<u16> = get_available_port();
        match port {
            Some(val) => {
                let server_dir = generate_unique_directory(working_dir);
                create_dedicated_server_environment(&server_dir);
                match start_dedicated_server(val, &server_dir) {
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

fn handle_connection(mut stream: TcpStream, working_dir: &str) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    if http_request.len() == 1 {
        let response = process_one_line_request(http_request, working_dir);

        match response {
            Some(val) => stream.write_all(val.as_bytes()).unwrap(),
            None => {}
        }
    } else {
        println!("Unknown request: {:#?}", http_request);
    }
}

fn get_available_port() -> Option<u16> {
    (8000..9000).find(|port| port_is_available(*port))
}

fn port_is_available(port: u16) -> bool {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}
