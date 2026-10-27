use crate::model::*;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, Semaphore};
include!(concat!(env!("OUT_DIR"), "/free.rs"));
const MAX_BYTES: usize = 2 * 1024 * 1024;
#[derive(Clone)]
struct OAuth {
    value: String,
    until: Instant,
}
#[derive(Default)]
struct Cache {
    oauth: Option<OAuth>,
    kits: Option<(Instant, Vec<Kit>)>,
    data: Option<(Instant, String, Vec<Icon>)>,
}
#[derive(Clone)]
pub struct Api {
    client: Client,
    endpoint: String,
    scopes: Arc<Mutex<HashMap<String, Arc<Mutex<Cache>>>>>,
    gate: Arc<Semaphore>,
}
impl Api {
    pub fn new() -> Result<Self, String> {
        Self::client(Client::builder(), "https://api.fontawesome.com")
    }
    fn client(builder: reqwest::ClientBuilder, endpoint: &str) -> Result<Self, String> {
        let client = builder
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(18))
            .user_agent("tezbar-fontawesome-v2/2.0.0")
            .build()
            .map_err(|_| "HTTPS client unavailable")?;
        Ok(Self {
            client,
            endpoint: endpoint.into(),
            scopes: Arc::default(),
            gate: Arc::new(Semaphore::new(2)),
        })
    }
    async fn request(
        &self,
        token: &str,
        token_request: bool,
        body: Option<Value>,
    ) -> Result<Value, String> {
        let _permit = self.gate.acquire().await.map_err(|_| "Network closed")?;
        let url = if token_request {
            format!("{}/token", self.endpoint)
        } else {
            self.endpoint.clone()
        };
        let mut request = self
            .client
            .post(url)
            .bearer_auth(token)
            .header("Accept", "application/json");
        if let Some(body) = body {
            request = request.json(&body);
        }
        let mut response = request
            .send()
            .await
            .map_err(|_| "Font Awesome network request failed. Retry deliberately.")?;
        if !response.status().is_success() {
            return Err(match response.status(){StatusCode::UNAUTHORIZED|StatusCode::FORBIDDEN=>"Font Awesome API token is invalid or lacks permissions. Open Extension Preferences.",StatusCode::TOO_MANY_REQUESTS=>"Font Awesome rate limit reached. Wait before Retry.",_=>"Font Awesome API request failed. Retry deliberately."}.into());
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_BYTES as u64)
        {
            return Err("Font Awesome response exceeds limit".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Font Awesome response interrupted")?
        {
            if bytes.len() + chunk.len() > MAX_BYTES {
                return Err("Font Awesome response exceeds limit".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid Font Awesome JSON")?;
        let mut todo = vec![&value];
        let mut count = 0;
        while let Some(item) = todo.pop() {
            count += 1;
            if count > 40000 {
                return Err("Font Awesome JSON node limit".into());
            }
            match item {
                Value::Array(a) => todo.extend(a),
                Value::Object(o) => todo.extend(o.values()),
                _ => {}
            }
        }
        if value
            .get("errors")
            .is_some_and(|e| !matches!(e,Value::Array(a) if a.is_empty()))
        {
            return Err(
                "Font Awesome GraphQL request failed. No partial results displayed.".into(),
            );
        }
        // Error messages never incorporate response bodies, credentials or reqwest URLs.
        if !token_request && contains_sensitive(&value, token) {
            return Err("Font Awesome response contained sensitive material".into());
        }
        Ok(value)
    }
    pub async fn fetch(
        &self,
        prefs: &Preferences,
        scope: &str,
        query: &str,
        style: &str,
        force: bool,
    ) -> Result<(Vec<Icon>, Vec<Kit>), String> {
        tokio::time::timeout(
            Duration::from_secs(50),
            self.fetch_inner(prefs, scope, query, style, force),
        )
        .await
        .map_err(|_| "Font Awesome request deadline exceeded. Retry deliberately.".to_string())?
    }
    async fn fetch_inner(
        &self,
        prefs: &Preferences,
        scope: &str,
        query: &str,
        style: &str,
        force: bool,
    ) -> Result<(Vec<Icon>, Vec<Kit>), String> {
        let cell = {
            let mut scopes = self.scopes.lock().await;
            if !scopes.contains_key(scope) && scopes.len() >= 8 {
                scopes.clear();
            }
            scopes.entry(scope.into()).or_default().clone()
        };
        let mut cache = cell.lock().await;
        if cache
            .oauth
            .as_ref()
            .is_none_or(|o| Instant::now() >= o.until)
        {
            cache.oauth = None;
            cache.data = None;
            cache.kits = None;
            let start = Instant::now();
            let value = self.request(&prefs.token, true, None).await?;
            let token = value["access_token"]
                .as_str()
                .filter(|t| {
                    !t.is_empty() && t.len() <= 2048 && t.bytes().all(|b| b.is_ascii_graphic())
                })
                .ok_or("Invalid access token response")?;
            let seconds = value["expires_in"]
                .as_u64()
                .filter(|s| *s > 0)
                .ok_or("Invalid access token expiry")?
                .min(3600);
            if prefs.pro {
                let scopes = value["scopes"]
                    .as_array()
                    .ok_or("Font Awesome token is missing permissions")?;
                if !["kits_read", "domains_read", "svg_icons_pro"]
                    .iter()
                    .all(|required| scopes.iter().any(|s| s.as_str() == Some(required)))
                {
                    return Err("Font Awesome API token is missing permissions: Read kits data, Allowed domains, Pro icons and metadata. Open Extension Preferences.".into());
                }
            }
            let until = start + Duration::from_secs(seconds);
            if Instant::now() >= until {
                return Err("Access token expired during acquisition".into());
            }
            cache.oauth = Some(OAuth {
                value: token.into(),
                until,
            });
        }
        let token = cache.oauth.as_ref().unwrap().value.clone();
        let result = async {
            let kits = if prefs.pro {
                if force
                    || cache
                        .kits
                        .as_ref()
                        .is_none_or(|(at, _)| at.elapsed() >= Duration::from_secs(300))
                {
                    let v = self.request(&token, false, Some(kits_body(false))).await?;
                    let kits = parse_kits(&v)?;
                    cache.kits = Some((Instant::now(), kits));
                }
                filtered_kits(&cache.kits.as_ref().unwrap().1, &prefs.filter)
            } else {
                Vec::new()
            };
            let key = digest(&serde_json::json!([query, style]).to_string());
            if !force {
                if let Some((at, k, icons)) = &cache.data {
                    if *k == key && at.elapsed() < Duration::from_secs(300) {
                        return Ok((icons.clone(), kits));
                    }
                }
            }
            let icons = if style.starts_with("kit:") {
                if !prefs.pro || !kits.iter().any(|k| k.opaque_id() == style) {
                    return Err("Selected kit is no longer available".into());
                }
                let v = self.request(&token, false, Some(kits_body(true))).await?;
                kit_icons(&parse_kits(&v)?, style, query)?
            } else {
                let v = self
                    .request(&token, false, Some(search_body(query, style)?))
                    .await?;
                let rows = v["data"]["search"]
                    .as_array()
                    .filter(|a| a.len() <= 48)
                    .ok_or("Invalid search response or row limit")?;
                let icons: Vec<Icon> = serde_json::from_value(Value::Array(rows.clone()))
                    .map_err(|_| "Invalid search icon metadata")?;
                validate_icons(&icons)?;
                icons.into_iter().filter(|i| !i.svgs.is_empty()).collect()
            };
            // Also reject the long-lived credential if a server reflects it in labels or SVG copy text.
            if icons.iter().any(|icon| {
                icon.id.contains(&prefs.token)
                    || icon.unicode.contains(&prefs.token)
                    || icon.svgs.iter().any(|svg| {
                        svg.html.contains(&prefs.token)
                            || svg.family_style.prefix.contains(&prefs.token)
                    })
            }) || kits
                .iter()
                .any(|k| k.name.contains(&prefs.token) || k.name.contains(&token))
            {
                return Err("Font Awesome response contained sensitive material".into());
            }
            cache.data = Some((Instant::now(), key, icons.clone()));
            Ok((icons, kits))
        }
        .await;
        if result.is_err() {
            cache.data = None;
            cache.kits = None;
            cache.oauth = None;
        }
        result
    }
}
fn parse_kits(v: &Value) -> Result<Vec<Kit>, String> {
    let rows = v["data"]["me"]["kits"]
        .as_array()
        .filter(|a| a.len() <= 128)
        .ok_or("Invalid kits response or kit limit")?;
    let kits: Vec<Kit> =
        serde_json::from_value(Value::Array(rows.clone())).map_err(|_| "Invalid kit metadata")?;
    for kit in &kits {
        if kit.name.is_empty()
            || kit.name.len() > 256
            || kit.name.chars().any(char::is_control)
            || kit.token.is_empty()
            || kit.token.len() > 256
            || kit.uploads.len() > 512
        {
            return Err("Invalid kit metadata bounds".into());
        }
    }
    Ok(kits)
}
#[cfg(test)]
impl Api {
    pub(crate) fn fixture(endpoint: &str, certificate: Option<&[u8]>) -> Self {
        assert!(
            endpoint.starts_with("http://127.0.0.1:") || endpoint.starts_with("https://localhost:")
        );
        let mut builder = Client::builder();
        if let Some(der) = certificate {
            builder = builder.add_root_certificate(reqwest::Certificate::from_der(der).unwrap());
        }
        Self::client(builder, endpoint).unwrap()
    }
}
#[cfg(test)]
impl Api {
    pub(crate) async fn expire_fixture(&self, scope: &str) {
        let cell = self.scopes.lock().await.get(scope).unwrap().clone();
        let mut cache = cell.lock().await;
        let oauth = cache.oauth.as_mut().unwrap();
        assert!(oauth.until <= Instant::now() + Duration::from_secs(3600));
        oauth.until = Instant::now();
    }
}

fn contains_sensitive(value: &Value, secret: &str) -> bool {
    let mut todo = vec![value];
    while let Some(value) = todo.pop() {
        match value {
            Value::String(s) if s.contains(secret) => return true,
            Value::Array(a) => todo.extend(a),
            Value::Object(o) => {
                if o.keys().any(|key| key.contains(secret)) {
                    return true;
                }
                todo.extend(o.values());
            }
            _ => {}
        }
    }
    false
}
