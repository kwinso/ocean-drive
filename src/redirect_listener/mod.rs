use std::{
    io::prelude::*,
    net::{TcpListener, TcpStream},
};
use anyhow::{Result, bail};

fn handle_request(mut stream: TcpStream) -> Option<String> {
    let mut buffer = [0; 1000];
    let _ = stream.read(&mut buffer).unwrap();

    // convert buffer into string and 'parse' the URL
    match String::from_utf8(buffer.to_vec()) {
        Ok(request) => {
            let split: Vec<&str> = request.split_whitespace().collect();

            if split.len() > 1 {
                success_res(stream);
                return Some(split[1].to_string());
            }

            error_res("Malformed request".to_string(), stream);
        }
        Err(e) => {
            error_res(format!("Invalid UTF-8 sequence: {}", e), stream);
        }
    };

    None
}

pub fn get_callback() -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:8080");

    match listener {
        Ok(listener) => {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        if let Some(url) = handle_request(stream) {
                            return Ok(url);
                        }
                    }
                    Err(e) => eprintln!("Error: {}", e)
                };
            }
        }
        Err(e) => bail!("Unable to setup listener on port 8080 for getting authorization code.\nError info: {}", e)
    }

    bail!("App was unable to get authorization code from Google API");
}

fn success_res(mut stream: TcpStream) {
    let contents = include_str!("success.html");
    let response = format!("HTTP/1.1 200 OK\r\n\r\n{}", contents);
  
    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn error_res(error_message: String, mut stream: TcpStream) {
    println!("Error: {}", error_message);
    let response = format!(
      "HTTP/1.1 400 Bad Request\r\n\r\n400 - Bad Request - {}\n",
      error_message
    );
  
    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
  }