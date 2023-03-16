use std::collections::HashMap;
use std::fs;
use std::process;
use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap_or_else(|err| {
        println!("{err}");
        process::exit(1);
    });

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);

        println!("Connection established!");
    }
}

type Handler = fn() -> String;

/// Gets the route from the request string, e.g. "/foo/bar?baz=qux" -> "/foo/bar"
///
/// # Examples
///
/// ```
/// let request = "GET /foo/bar?baz=qux HTTP/1.1";
/// let route = get_route(request);
/// assert_eq!(route, "/foo/bar");
/// ```
fn get_route(request: &str) -> String {
    let mut parts = request.split_whitespace();
    let _method = parts.next().unwrap();
    let route = parts.next().unwrap();
    let _version = parts.next().unwrap();

    let mut route_parts = route.split('?');
    let route = route_parts.next().unwrap();

    route.to_string()
}

/// Handles a connection, reading the request and writing the response.
fn handle_connection(mut stream: TcpStream) {
    let buf = BufReader::new(&mut stream);
    let req: Vec<_> = buf
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    let mut status = "HTTP/1.1 200 OK \r\n";
    let request_string = match req.first() {
        Some(r) => r.to_string(),
        None => {
            println!("Request {req:#?}");
            status = "HTTP/1.1 404 Not Found \r\n";
            "/404".to_string()
        }
    };
    let request_route = get_route(&request_string);

    let mut handlers: HashMap<&str, Handler> = HashMap::new();
    handlers.insert("/", index);

    let handler = match handlers.get(request_route.as_str()) {
        Some(h) => h.to_owned(),
        None => {
            println!("Request {req:#?}");
            status = "HTTP/1.1 404 Not Found \r\n";
            not_found
        }
    };

    println!("Request {req:#?}");
    let body = handler();
    let size = format!("Content-Length: {}\r\n", body.len());
    let response = format!("{status}{size}\r\n{body}");
    match stream.write_all(response.as_bytes()) {
        Ok(r) => r,
        Err(err) => {
            println!("{err}");
        }
    }
    println!("Request {:#?}", req[0]);
}

fn index() -> String {
    match fs::read_to_string("src/index.html") {
        Ok(r) => r,
        Err(err) => {
            println!("{err}");
            "".to_string()
        }
    }
}

fn not_found() -> String {
    match fs::read_to_string("src/404.html") {
        Ok(r) => r,
        Err(err) => {
            println!("{err}");
            "".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_route() {
        let request = "GET / HTTP/1.1";
        let route = get_route(request);
        assert_eq!(route, "/");

        let request = "GET /foo HTTP/1.1";
        let route = get_route(request);
        assert_eq!(route, "/foo");

        let request = "GET /foo/bar HTTP/1.1";
        let route = get_route(request);
        assert_eq!(route, "/foo/bar");

        let request = "GET /foo/bar?baz=qux HTTP/1.1";
        let route = get_route(request);
        assert_eq!(route, "/foo/bar");
    }
}
