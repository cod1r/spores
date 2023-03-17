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
fn get_parsed_request(request: &[String]) -> ParsedRequest {
    println!("{request:#?}");
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
            if !header.contains(':') || header.starts_with('{') {
                continue;
            }
            let mut split = header.split(':');
            headers.insert(
                // key
                split.next().unwrap().trim().to_string(),
                // value
                split.collect::<Vec<&str>>().join(":").trim().to_string(),
            );
        }
    }

    let method = match method {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        _ => Method::GET,
    };

    let body = match method {
        Method::POST => match request.last() {
            Some(r) => {
                if r.starts_with('{') {
                    r
                } else {
                    ""
                }
            }
            None => "",
        },
        _ => "",
    };

    ParsedRequest {
        method,
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
    let mut reader = BufReader::new(&stream);
    let mut req: Vec<String> = reader
        .by_ref()
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    match req[0].contains("POST") {
        true => {
            println!("POST request");
            let mut contents_raw: Vec<u8> = vec![];
            reader.read_until(b'}', &mut contents_raw).unwrap();
            req.push(String::from_utf8(contents_raw).unwrap());
        }
        false => {
            println!("GET request");
        }
    };

    let request_route = get_parsed_request(&req);

    println!("Request Route {request_route:#?}");

    let mut handlers: HashMap<&str, Handler> = HashMap::new();
    handlers.insert("/", index);
    let mut status = "HTTP/1.1 200 OK \r\n";
    let handler = match handlers.get(request_route.route.as_str()) {
        Some(h) => h.to_owned(),
        None => {
            println!("Request {req:#?}");
            status = "HTTP/1.1 404 Not Found \r\n";
            not_found
        }
    };

    let body = handler();
    let response = format!("{status}Content-Length: {}\r\n\r\n{body}", body.len());
    match stream.write_all(response.as_bytes()) {
        Ok(r) => r,
        Err(err) => {
            println!("{err}");
        }
    }
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
        assert_eq!(parsed.headers.get("Host").unwrap(), "localhost:7878");
    }
}
