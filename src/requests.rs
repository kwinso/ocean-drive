use reqwest::{header::HeaderMap, Method, Response};

// todo: make struct to create single object that will handle basic configuration.
// todo: add query param to every request to encode all variables

pub async fn get<'de, T>(
    uri: String,
    headers: Vec<(&'static str, &'static str)>,
) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let client = reqwest::Client::new();
    let mut header_map = HeaderMap::new();

    for h in headers.iter() {
        header_map.insert(h.0, h.1.parse().unwrap());
    }

    match client.get(uri.clone()).headers(header_map).send().await {
        Ok(resp) => match resp.json::<T>().await {
            Ok(r) => { Ok(r)},
            Err(e) => Err(format!("Unable to deserialize HTTP response.\n{}", e)),
        },
        Err(e) => Err(format!("Failed to GET '{}'.\n{}", uri, e)),
    }
}

pub async fn post<'de, T>(
    uri: String,
    body: String,
    headers: Vec<(&'static str, &'static str)>,
) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let client = reqwest::Client::new();
    let mut header_map = HeaderMap::new();

    for h in headers {
        header_map.insert(h.0, h.1.parse().unwrap());
    }

    match client
        .post(uri.clone())
        .headers(header_map)
        .body(body)
        .send()
        .await
    {
        Ok(resp) => match resp.json::<T>().await {
            Ok(r) => Ok(r),
            Err(e) => Err(format!("Unable to deserialize HTTP response. \n{}", e)),
        },
        Err(e) => Err(format!("Failed to POST '{}'.\n{}", uri, e)),
    }
}

pub async fn post_json<'de, T>(uri: String, json_body: T) -> Result<T, String>
where
    T: serde::de::DeserializeOwned + Into<json::JsonValue>,
{
    if let Some(parsed) = json::from(json_body).as_str() {
        return post(
            uri,
            parsed.to_string(),
            vec![("Content-Type", "application/json")],
        )
        .await?;
    }

    Err("Failed to create JSON String".to_string())
}
