use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                handle_connection(s);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    if let Some(path) = request_line.split_whitespace().nth(1) {
        if path == "/" {
            stream
                .write_all("HTTP/1.1 200 OK\r\n\r\n".as_bytes())
                .unwrap();
        } else if let Some(echo_str) = path.strip_prefix("/echo/") {
            let status_line = "HTTP/1.1 200 OK";
            let content_type = "text/plain";
            let content_length = 3;
            let response = format!(
                "{status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {content_length}\r\n\r\n{echo_str}"
            );
            stream.write_all(response.as_bytes()).unwrap();
        } else {
            stream
                .write_all("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
                .unwrap();
        }
    }
}
