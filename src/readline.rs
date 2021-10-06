/* Some functions to get user input conveniently */

use std::io;
use std::io::prelude::*;

/* This function returns None only if empty line was supplied (user just hit Enter key) */
pub fn prompt(prompt: &str) -> Option<String> {
    print!("{}: ", prompt);
    io::stdout().flush().unwrap();
    let mut ans = String::new();

    if let Err(_) = io::stdin().read_line(&mut ans) {
        return None;
    }

    if ans.trim().is_empty() {
        return None;
    }

    Some(ans.trim().to_string())
}

pub fn promt_default(text: &str, default: &str) -> String {
    if let Some(ans) = prompt(&format!("{} (Default: {:?})", text, default)) {
        return ans;
    }

    return default.to_string();
}

pub fn binary_prompt(text: &str) -> bool {
    loop {
        if let Some(ans) = prompt(&format!("{} (Y/N))", text)) {
            match ans.to_lowercase().as_str() {
                "y" => true,
                "n" => false,
                _ => continue,
            };
        }

        return true;
    }
}
