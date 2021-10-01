use std::collections::HashMap;
use urlencoding;
use regex::Regex;
use anyhow::{Result, bail};

pub fn get_query(url: String) -> Result<HashMap<String, String>> {
    let url_re = Regex::new(r".*\?").unwrap();
    let query = url_re.replace(&url, "");
    let mut vars: HashMap<String, String> = HashMap::new();

    for pair in query.split("&") {
        let var = pair.split("=").collect::<Vec<&str>>();
        if var.len() == 2 {
            let value = urlencoding::decode(var[1])?;
            vars.insert(var[0].to_string(), value.to_string());
            continue;
        }
        bail!("Unable to process URL Query: malformed URL");
    }

    Ok(vars)
}