use std::io;
use std::io::prelude::*;

pub fn prompt(prompt: &str) -> Result<String, ()> {
    print!("{}: ", prompt);
    io::stdout().flush().unwrap();

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        return Ok(line.expect("Unable to parse this text"));
    }

    Err(())
}