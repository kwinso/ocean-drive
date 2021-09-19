use std::collections::HashMap;
use regex::Regex;

pub fn get_query(url: String) -> Result<HashMap<String, String>, String> {
    let url_re = Regex::new(r".*\?").unwrap();
    let query = url_re.replace(&url, "");
    let mut vars: HashMap<String, String> = HashMap::new();

    for pair in query.split("&") {
        let var = pair.split("=").collect::<Vec<&str>>();
        if var.len() == 2 {
            vars.insert(var[0].to_string(), var[1].to_string());
            continue;
        }
        return Err(String::from("Malformed URL"));
    }

    Ok(vars)
}