/* Some functions to get user input conveniently */

use std::io;
use std::io::prelude::*;

pub fn prompt(prompt: &str) -> Option<String> {
    print!("{}: ", prompt);
    io::stdout().flush().unwrap();

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        if let Ok(ans) = line {
            if !ans.is_empty() {
                return Some(ans)
            }
        }

        return None;
    }

    None
}

pub fn promt_default(prompt_text: &str, default: &str) -> String {
    if let Some(ans) = prompt(&format!("{} (Default: {:?})", prompt_text, default)) {
        return ans;
    }

    return default.to_string();
}