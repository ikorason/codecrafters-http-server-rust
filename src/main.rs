use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                handle_connection(s);
            }
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    if let Some(response) = parse_and_generate_response(&stream) {
        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    }
}

fn parse_and_generate_response(stream: &TcpStream) -> Option<String> {
    let (request_line, headers) = parse_request(stream);

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return None; // invalid request
    }

    let path = parts[1];

    if path == "/" {
        Some(String::from("HTTP/1.1 200 OK\r\n\r\n"))
    } else if let Some(echo_str) = path.strip_prefix("/echo/") {
        Some(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n",
            echo_str.len()
        ))
    } else if path == "/user-agent" {
        let user_agent = headers.get("User-Agent").unwrap();
        Some(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n{}",
            user_agent.len(),
            user_agent,
        ))
    } else {
        Some(String::from("HTTP/1.1 404 Not Found\r\n\r\n"))
    }
}

fn parse_request(stream: &TcpStream) -> (String, HashMap<String, String>) {
    let mut reader = BufReader::new(stream);

    // read the request line
    let mut request_line = String::new();
    reader.read_line(&mut request_line).unwrap();
    let request_line = request_line.trim_end();

    // parse headers
    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let line = line.trim_end();

        if line.is_empty() {
            break;
        }

        if let Some(index) = line.find(':') {
            let k = line[..index].trim().to_string();
            let v = line[index + 1..].trim().to_string();
            headers.insert(k, v);
        }
    }

    (request_line.to_string(), headers)
}
