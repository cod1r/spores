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
/// let route = get_parsed_request(request);
/// assert_eq!(route, "/foo/bar");
/// ```
fn get_parsed_request(request: &Vec<String>) -> ParsedRequest {
    let request_line = match request.first() {
        Some(r) => r,
        None => "",
    };
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap();
    let route = parts.next().unwrap();
    let version = parts.next().unwrap();

    let mut route_parts = route.split('?');
    let route = route_parts.next().unwrap();

    let query = route_parts.next().unwrap_or("");

    let mut headers = HashMap::new();
    for (index, header) in request.iter().enumerate() {
        if index > 0 {
            let mut split = header.split(':');
            headers.insert(
                split.next().unwrap().to_string(),
                split.next().unwrap().to_string(),
            );
        }
    }

    let body = match request.last() {
        Some(b) => b,
        None => "",
    };

    ParsedRequest {
        method: match method {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            _ => Method::GET,
        },
        route: route.to_string(),
        version: version.to_string(),
        query: query.to_string(),
        headers,
        body: body.to_string(),
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, PartialEq)]
enum Method {
    GET,
    POST,
    PUT,
    DELETE,
}

#[derive(Debug, PartialEq)]
struct ParsedRequest {
    method: Method,
    route: String,
    version: String,
    query: String,
    headers: HashMap<String, String>,
    body: String,
}

/// Handles a connection, reading the request and writing the response.
fn handle_connection(mut stream: TcpStream) {
    let buf = BufReader::new(&mut stream);
    let req: Vec<String> = buf
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    let request_route = get_parsed_request(&req);

    let mut handlers: HashMap<&str, Handler> = HashMap::new();
    handlers.insert("/", index);
    let mut status = "GET / HTTP/1.1 \r\n";
    let handler = match handlers.get(request_route.route.as_str()) {
        Some(h) => h.to_owned(),
        None => {
            println!("Request {req:#?}");
            status = "HTTP/1.1 404 Not Found \r\n";
            not_found
        }
    };

    println!("Request {req:#?}");
    let body = handler();
    let response = format!("{status}Content-Length: {}\r\n\r\n{body}", body.len());
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
    fn test_get_parsed_request() {
        let request = vec!["GET / HTTP/1.1".to_string()];
        let parsed = get_parsed_request(&request);
        assert_eq!(parsed.route, "/");

        let request = vec!["GET /foo HTTP/1.1".to_string()];
        let parsed = get_parsed_request(&request);
        assert_eq!(parsed.route, "/foo");

        let request = vec!["GET /foo/bar HTTP/1.1".to_string()];
        let parsed = get_parsed_request(&request);
        assert_eq!(parsed.route, "/foo/bar");

        let request = vec!["GET /foo/bar?baz=qux HTTP/1.1".to_string()];
        let parsed = get_parsed_request(&request);
        assert_eq!(parsed.route, "/foo/bar");

        let request = vec![
            "GET /foo/bar?baz=qux HTTP/1.1".to_string(),
            "Host: localhost:7878".to_string(),
        ];
        let parsed = get_parsed_request(&request);
        assert_eq!(parsed.headers.get("Host").unwrap(), " localhost:7878");
    }
}
