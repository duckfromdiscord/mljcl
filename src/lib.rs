pub mod json;

#[cfg(feature = "full")]
pub mod types;

#[cfg(feature = "full")]
pub mod charts;

#[cfg(feature = "full")]
pub mod history;

#[cfg(feature = "full")]
pub mod range;

#[cfg(feature = "full")]
pub mod art;

use crate::json::{ScrobbleReq, ScrobbleRes};
use std::str::FromStr;
use std::collections::HashMap;

use reqwest::{Client, header::{HeaderMap, HeaderName, HeaderValue}};

#[derive(Debug)]
pub enum RequestError {
    LocalError(reqwest::Error),
    ServerError(String),
}

#[derive(Debug, Clone)]
pub struct MalojaCredentials {
    pub https: bool,
    pub skip_cert_verification: bool,
    pub ip: String,
    pub port: u16,
    pub path: Option<String>,
    pub headers: Option<HashMap<String,String>>,
    pub api_key: Option<String>,
}

impl MalojaCredentials {
    pub fn get_url(&self) -> String {
        let protocol = match self.https {
            true => "https://",
            false => "http://",
        };
        let mut sub_path = self.clone().path.unwrap_or("".to_string()).trim_matches('/').to_owned();
        if !sub_path.is_empty() {
            sub_path = "/".to_owned() + &sub_path;
        }
        format!("{}{}:{}{}", protocol, &self.ip, &self.port, sub_path)
    }
}

pub fn full_query_path<T: for<'de> serde::Serialize>(query: T, path: &str) -> String {
    let qs = serde_qs::to_string(&query).unwrap();
    match qs.is_empty() {
        true => {
            path.to_string()
        },
        false => {
            path.to_string() + "?" + &qs
        }
    }
}

pub fn parse_headers(maybe_headers: Option<HashMap<String, String>>) -> HeaderMap {
    let mut map = HeaderMap::new();
    if let Some(headers) = maybe_headers {
        for key in headers.keys() {
            let header_key = HeaderName::from_str(key);
            let header_value = HeaderValue::from_str(headers.get(key).unwrap());
            if header_key.is_err() || header_value.is_err() {
                continue;
            }
            map.insert(header_key.unwrap(), header_value.unwrap());
        }
    }
    map
}

async fn handle_response<T: crate::json::MalojaResponse + for<'de> serde::Deserialize<'de>>(response: Result<reqwest::Response, reqwest::Error>) -> Result<T, RequestError> {
    if response.is_err() {
        return Err(RequestError::LocalError(response.err().unwrap()));
    }
    let response = response.unwrap();
    match response.json::<T>().await {
        Err(error) => {
            Err(RequestError::LocalError(error))
        },
        Ok(parsed_response) => {
            match parsed_response.get_error() {
                None => Ok(parsed_response),
                Some(error) => Err(RequestError::ServerError(error.desc)),
            }
        }
    }
}

pub fn get_client_async(credentials: &MalojaCredentials) -> reqwest::Client {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(credentials.skip_cert_verification)
        .build()
        .unwrap()
}

pub async fn scrobble_async(title: String, artist: String, credentials: MalojaCredentials, client: Client) -> Result<ScrobbleRes, RequestError> {
    let scrobblebody = ScrobbleReq {
        artist: Some(artist),
        artists: None,
        title,
        album: None,
        albumartists: None,
        duration: None,
        length: None,
        time: None,
        key: credentials.api_key.as_ref().unwrap().to_string(),
    };
    let response = client
        .post(credentials.get_url() + "/apis/mlj_1/newscrobble")
        .headers(parse_headers(credentials.headers))
        .json(&scrobblebody)
        .send()
        .await;
    handle_response::<ScrobbleRes>(response).await
}

pub fn scrobble(title: String, artist: String, credentials: MalojaCredentials) -> Result<ScrobbleRes, RequestError> { 
    tokio::runtime::Runtime::new().unwrap().block_on( async {
        let client = get_client_async(&credentials);
        scrobble_async(title, artist, credentials, client).await
    })
}