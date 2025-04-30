use std::{
    collections::HashMap,
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    thread,
};

use flate2::{write::GzEncoder, Compression};

fn main() {
    let mut args = env::args();
    let _program = args.next();

    let mut base_dir = None;
    while let Some(arg) = args.next() {
        if arg == "--directory" {
            if let Some(path) = args.next() {
                base_dir = Some(PathBuf::from(path));
            }
        }
    }

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let base_dir = base_dir.clone();

        thread::spawn(move || {
            handle_connection(stream, base_dir);
        });
    }
}

fn handle_connection(mut stream: TcpStream, base_dir: Option<PathBuf>) {
    while let Some(response) = parse_and_generate_response(&mut stream, base_dir.clone()) {
        stream.write_all(&response).unwrap();
        stream.flush().unwrap();
    }
}

fn parse_and_generate_response(
    stream: &mut TcpStream,
    base_dir: Option<PathBuf>,
) -> Option<Vec<u8>> {
    let (request_line, headers, mut reader) = parse_request(stream);

    if let Some(conn) = headers.get("Connection") {
        if conn.eq_ignore_ascii_case("close") {
            return None;
        }
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return None; // invalid request
    }

    let method = parts[0];
    let path = parts[1];

    match method {
        "GET" => {
            if path == "/" {
                return Some(b"HTTP/1.1 200 OK\r\n\r\n".to_vec());
            }

            if let Some(echo_str) = path.strip_prefix("/echo/") {
                let mut response_body = echo_str.as_bytes().to_vec();
                let mut response_headers =
                    String::from("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n");

                if let Some(accept_encoding) = headers.get("Accept-Encoding") {
                    let encoding_is_gzip =
                        accept_encoding.split(',').any(|enc| enc.trim() == "gzip");

                    if encoding_is_gzip {
                        let mut encoder = GzEncoder::new(vec![], Compression::default());
                        encoder.write_all(&response_body).unwrap();
                        response_body = encoder.finish().unwrap();
                        response_headers.push_str("Content-Encoding: gzip\r\n");
                    }
                }

                response_headers
                    .push_str(&format!("Content-Length: {}\r\n\r\n", response_body.len()));

                let mut response = response_headers.into_bytes();
                response.extend_from_slice(&response_body);
                return Some(response);
            }

            if let Some(file_path) = path.strip_prefix("/files/") {
                if file_path.contains("..") {
                    return Some(b"HTTP/1.1 400 Bad Request\r\n\r\n".to_vec());
                }

                let base_dir = base_dir?;
                let full_path = base_dir.join(file_path);

                match full_path.canonicalize() {
                    Ok(resolved_path) => match fs::read(&resolved_path) {
                        Ok(content) => {
                            let mut headers = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
                                    content.len()
                                )
                                .into_bytes();
                            headers.extend_from_slice(&content);
                            Some(headers)
                        }
                        Err(_) => Some(b"HTTP/1.1 404 Not Found\r\n\r\n".to_vec()),
                    },
                    Err(_) => Some(b"HTTP/1.1 404 Not Found\r\n\r\n".to_vec()),
                }
            } else if path == "/user-agent" {
                if let Some(user_agent) = headers.get("User-Agent") {
                    let body = user_agent.as_bytes();
                    let mut response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                        body.len()
                    )
                    .into_bytes();
                    response.extend_from_slice(body);
                    Some(response)
                } else {
                    Some(b"HTTP/1.1 400 Bad Request\r\n\r\n".to_vec())
                }
            } else {
                Some(b"HTTP/1.1 404 Not Found\r\n\r\n".to_vec())
            }
        }
        "POST" => {
            if let Some(file_path) = path.strip_prefix("/files/") {
                let content_length = headers
                    .get("Content-Length")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);

                let mut body = vec![0; content_length];
                reader.read_exact(&mut body).unwrap();

                if file_path.contains("..") {
                    return Some(b"HTTP/1.1 400 Bad Request\r\n\r\n".to_vec());
                }

                let base_dir = base_dir?;
                let full_path = base_dir.join(file_path);

                match fs::write(&full_path, &body) {
                    Ok(_) => Some(b"HTTP/1.1 201 Created\r\n\r\n".to_vec()),
                    Err(_) => Some(b"HTTP/1.1 500 Internal Server Error\r\n\r\n".to_vec()),
                }
            } else {
                Some(b"HTTP/1.1 404 Not Found\r\n\r\n".to_vec())
            }
        }
        _ => Some(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n".to_vec()),
    }
}

fn parse_request(stream: &TcpStream) -> (String, HashMap<String, String>, BufReader<&TcpStream>) {
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

    (request_line.to_string(), headers, reader)
}
