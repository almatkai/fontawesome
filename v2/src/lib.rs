pub mod api;
pub mod model;
mod storage;
use api::Api;
use model::*;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};
use tezbar_extension_sdk::{BoxFuture, CoreResult, Extension, ExtensionContext};

#[derive(Clone)]
enum Intent {
    Copy(String, &'static str),
    Style,
    Retry,
    RetryEffect,
    Preferences,
}
#[derive(Clone)]
struct Effect {
    method: &'static str,
    params: Value,
    success: &'static str,
}
struct Session {
    prefs: Preferences,
    scope: String,
    query: String,
    style: String,
    revision: u64,
    icons: Vec<Icon>,
    kits: Vec<Kit>,
    intents: HashMap<String, Intent>,
    pending: Option<Effect>,
    notice: Option<String>,
    hud: Option<&'static str>,
}
type Registry = Arc<Mutex<HashMap<String, Session>>>;
pub struct Adapter {
    api: Api,
    sessions: Registry,
}
struct Cleanup {
    registry: Weak<Mutex<HashMap<String, Session>>>,
    id: String,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Some(r) = self.registry.upgrade() {
            if let Ok(mut r) = r.lock() {
                r.remove(&self.id);
            }
        }
    }
}
impl Adapter {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            api: Api::new()?,
            sessions: Arc::default(),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, HashMap<String, Session>>, String> {
        self.sessions
            .lock()
            .map_err(|_| "Session registry unavailable".into())
    }
    fn render(&self, id: &str, state: &mut Value) -> CoreResult {
        let mut registry = self.lock()?;
        let s = registry.get_mut(id).ok_or("Session closed")?;
        // A discarded SDK response must never leave its intents authorized.
        s.revision = s.revision.checked_add(1).ok_or("Revision limit")?;
        s.intents.clear();
        let mut actions = Vec::new();
        let mut children = Vec::new();
        let mut intent = |kind: Intent, title: &str| {
            let key = format!("{}:{}", s.revision, actions.len());
            s.intents.insert(key.clone(), kind);
            actions.push(json!({"id":key,"title":title,"kind":"action"}));
            key
        };
        let selector = intent(Intent::Style, "Select Family & Style");
        let order: Vec<usize> = if s.prefs.pro {
            (8..13).chain(0..8).chain(13..17).collect()
        } else {
            vec![8, 12]
        };
        let mut options: Vec<Value> = order
            .into_iter()
            .map(|i| {
                node(
                    "List.Dropdown.Item",
                    json!({"title":STYLES[i].1,"value":STYLES[i].0}),
                    vec![],
                )
            })
            .collect();
        options.extend(s.kits.iter().map(|k| {
            node(
                "List.Dropdown.Item",
                json!({"title":k.name,"value":k.opaque_id()}),
                vec![],
            )
        }));
        let mut copies = COPIES.to_vec();
        if let Some(i) = copies.iter().position(|(key, _)| *key == s.prefs.primary) {
            let p = copies.remove(i);
            copies.insert(0, p);
        }
        for (i, icon) in s.icons.iter().enumerate() {
            let mut ids = Vec::new();
            for (action, title) in &copies {
                let text = copy_text(icon, action)?;
                let key = intent(Intent::Copy(text, copy_hud(action)), title);
                ids.push(key);
            }
            let content = safe_image(&icon.svgs[0].html)
                .map(|source| json!({"value":{"source":source}}))
                .unwrap_or(json!({}));
            children.push(node(
                "Grid.Item",
                json!({"id":format!("row:{i}"),"title":icon.id,"content":content,"actionIds":ids}),
                vec![],
            ));
        }
        let retry = intent(Intent::Retry, "Retry Font Awesome Request");
        let preferences = intent(Intent::Preferences, "Open Extension Preferences");
        let mut utility_ids = vec![retry.clone(), preferences.clone()];
        if s.pending.is_some() {
            utility_ids.push(intent(
                Intent::RetryEffect,
                "Retry Prepared Effect — may repeat the previous effect",
            ));
        }
        if s.icons.is_empty() {
            children.push(node("Grid.EmptyView",json!({"title":s.notice.as_deref().unwrap_or("No icons found"),"description":"Use Actions to Retry or Open Extension Preferences.","actionIds":[retry,preferences]}),vec![]));
        }
        for child in &mut children {
            if child["type"] == "Grid.Item" {
                child["props"]["actionIds"]
                    .as_array_mut()
                    .unwrap()
                    .extend(utility_ids.iter().map(|id| json!(id)));
            }
        }
        actions.retain(|a| a["id"] != selector);
        let root = node(
            "Grid",
            json!({"columns":8,"inset":"large","filtering":false,"__hasServerSearch":true,"searchText":s.query,"searchBarPlaceholder":"Search icons...","error":s.notice,"searchBarAccessory":node("List.Dropdown",json!({"actionId":selector,"value":s.style}),options)}),
            children,
        );
        *state = json!({"revision":s.revision});
        let mut result = tezbar_extension_sdk::view("Search Font Awesome", root, actions);
        result["refreshMs"] = Value::Null;
        if let Some(hud) = s.hud.take() {
            result["effects"] = json!([{"kind":"hud","title":hud}]);
        }
        Ok(result)
    }
    async fn load(&self, ctx: &ExtensionContext, id: &str, force: bool) -> Result<(), String> {
        let (prefs, scope, query, style, revision) = {
            let mut r = self.lock()?;
            let s = r.get_mut(id).ok_or("Session closed")?;
            s.intents.clear();
            s.icons.clear();
            s.notice = Some("Search interrupted. Use Retry to try again.".into());
            s.revision += 1;
            (
                s.prefs.clone(),
                s.scope.clone(),
                s.query.clone(),
                s.style.clone(),
                s.revision,
            )
        };
        let response = tokio::select! {biased;_=ctx.cancelled()=>return Err("Operation cancelled".into()),result=self.api.fetch(&prefs,&scope,&query,&style,force)=>result};
        let mut r = self.lock()?;
        let s = r.get_mut(id).ok_or("Session closed")?;
        if s.revision != revision {
            return Err("Stale search result".into());
        }
        match response {
            Ok((icons, kits)) => {
                s.icons = icons;
                s.kits = kits;
                s.notice = None;
            }
            Err(e) => {
                s.notice = Some(e);
            }
        }
        Ok(())
    }
    async fn execute(
        &self,
        ctx: &ExtensionContext,
        id: &str,
        effect: Effect,
    ) -> Result<(), String> {
        // Persist intent in authoritative memory BEFORE awaiting. Cancellation/invalid ACK leaves an uncertain effect, never automatic replay.
        {
            let mut r = self.lock()?;
            let s = r.get_mut(id).ok_or("Session closed")?;
            s.pending = Some(effect.clone());
            s.intents.clear();
            s.revision += 1;
            s.notice = Some(
                "Previous effect may have completed. Explicit Retry Prepared Effect may repeat it."
                    .into(),
            );
        }
        let reply = tokio::time::timeout(
            std::time::Duration::from_secs(12),
            ctx.host(effect.method, effect.params.clone()),
        )
        .await;
        let success = matches!(reply,Ok(Ok(ref value)) if strict_ack(value));
        let mut r = self.lock()?;
        let s = r.get_mut(id).ok_or("Session closed")?;
        if success {
            s.pending = None;
            s.notice = None;
            s.hud = Some(effect.success);
        } else {
            s.notice=Some("Effect not acknowledged; it may have completed. Explicit Retry Prepared Effect may repeat it.".into());
        }
        Ok(())
    }
}
pub fn strict_ack(v: &Value) -> bool {
    v.as_object().is_some_and(|o| {
        o.get("ok") == Some(&Value::Bool(true))
            && o.iter()
                .all(|(k, v)| k == "ok" || (k == "message" && v.is_string()))
    })
}
pub fn node(kind: &str, props: Value, children: Vec<Value>) -> Value {
    json!({"type":kind,"props":props,"children":children})
}
impl Extension for Adapter {
    fn id(&self) -> &'static str {
        "com.tezbar.native.fontawesome"
    }
    fn version(&self) -> &'static str {
        "2.0.0"
    }
    fn commands(&self) -> &'static [&'static str] {
        &["index"]
    }
    fn handle<'a>(
        &'a self,
        ctx: &'a ExtensionContext,
        _command: &'a str,
        op: &'a str,
        payload: &'a Value,
        state: &'a mut Value,
    ) -> BoxFuture<'a, CoreResult> {
        Box::pin(async move {
            let id = payload["sessionId"]
                .as_str()
                .ok_or("Missing session identity")?;
            if op == "extension:dispose-session" {
                self.lock()?.remove(id);
                return Ok(tezbar_extension_sdk::no_view("Closed"));
            }
            if op == "extension:run-command" {
                let prefs = Preferences::parse(&payload["preferences"])?;
                let scope = prefs.scope(&ctx.paths.data_dir.to_string_lossy());
                let query = payload["searchText"].as_str().unwrap_or("").to_owned();
                if query.len() > 1024 {
                    return Err("Search exceeds 1024 bytes".into());
                }
                let style = prefs.style.clone();
                {
                    let mut r = self.lock()?;
                    if r.len() >= 128 || r.contains_key(id) {
                        return Err("Session capacity or duplicate identity".into());
                    }
                    r.insert(
                        id.into(),
                        Session {
                            prefs: prefs.clone(),
                            scope: scope.clone(),
                            query,
                            style,
                            revision: 0,
                            icons: vec![],
                            kits: vec![],
                            intents: HashMap::new(),
                            pending: None,
                            notice: None,
                            hud: None,
                        },
                    );
                }
                let cleanup = Cleanup {
                    registry: Arc::downgrade(&self.sessions),
                    id: id.into(),
                };
                ctx.spawn(async move {
                    let _guard = cleanup;
                    std::future::pending::<()>().await;
                })?;
                if prefs.remember {
                    match storage::read(ctx) {
                        Ok(store) => {
                            if let Some(saved) = store[&scope].as_str() {
                                if valid_selection(saved, prefs.pro) {
                                    self.lock()?.get_mut(id).ok_or("Session closed")?.style =
                                        saved.into();
                                }
                            }
                        }
                        Err(_) => {
                            self.lock()?.get_mut(id).ok_or("Session closed")?.notice=Some("Remembered selection storage is corrupt. Use Retry or choose a style.".into());
                            return self.render(id, state);
                        }
                    }
                }
                self.load(ctx, id, false).await?;
                return self.render(id, state);
            }
            {
                let mut r = self.lock()?;
                let s = r.get_mut(id).ok_or("Session closed")?;
                if state["revision"].as_u64() != Some(s.revision) {
                    s.intents.clear();
                    s.icons.clear();
                    if s.pending.is_none() {
                        s.notice = Some(
                            "Previous operation was discarded. Use Retry to fetch current results."
                                .into(),
                        );
                    }
                }
            }
            match op {
                "extension:search-text-changed" => {
                    let query = payload["searchText"]
                        .as_str()
                        .ok_or("Missing search text")?;
                    if query.len() > 1024 {
                        return Err("Search exceeds 1024 bytes".into());
                    }
                    {
                        let mut r = self.lock()?;
                        let s = r.get_mut(id).ok_or("Session closed")?;
                        s.query = query.into();
                        s.pending = None;
                    }
                    self.load(ctx, id, false).await?;
                }
                "extension:refresh-session" => {}
                "extension:load-more" => return Ok(tezbar_extension_sdk::unchanged()),
                "extension:invoke-action" => {
                    let key = payload["actionId"]
                        .as_str()
                        .ok_or("Missing action identity")?;
                    let action = {
                        let mut r = self.lock()?;
                        let s = r.get_mut(id).ok_or("Session closed")?;
                        s.intents
                            .remove(key)
                            .ok_or("Action expired; refresh the current view")?
                    };
                    match action {
                        Intent::Copy(text, success) => {
                            self.execute(
                                ctx,
                                id,
                                Effect {
                                    method: "host.clipboard.write",
                                    params: json!({"text":text}),
                                    success,
                                },
                            )
                            .await?
                        }
                        Intent::Preferences => {
                            self.execute(
                                ctx,
                                id,
                                Effect {
                                    method: "host.settings.open",
                                    params: json!({"surface":"settings-extensions"}),
                                    success: "Opened Extension Preferences",
                                },
                            )
                            .await?
                        }
                        Intent::RetryEffect => {
                            let effect = self
                                .lock()?
                                .get_mut(id)
                                .ok_or("Session closed")?
                                .pending
                                .take()
                                .ok_or("No prepared effect")?;
                            self.execute(ctx, id, effect).await?;
                        }
                        Intent::Retry => {
                            self.load(ctx, id, true).await?;
                        }
                        Intent::Style => {
                            let value = payload["formValues"]["value"]
                                .as_str()
                                .ok_or("Missing style selection")?;
                            let (remember, scope) = {
                                let mut r = self.lock()?;
                                let s = r.get_mut(id).ok_or("Session closed")?;
                                if !valid_selection(value, s.prefs.pro)
                                    || (value.starts_with("kit:")
                                        && !s.kits.iter().any(|k| k.opaque_id() == value))
                                {
                                    return Err("Style or kit is unavailable".into());
                                }
                                s.style = value.into();
                                s.icons.clear();
                                s.intents.clear();
                                s.pending = None;
                                s.revision += 1;
                                (s.prefs.remember, s.scope.clone())
                            };
                            if remember {
                                let saved = value.to_owned();
                                if storage::save(ctx, &scope, &saved).is_err() {
                                    self.lock()?.get_mut(id).ok_or("Session closed")?.notice=Some("Could not save remembered selection. Choose again after repairing private storage.".into());
                                    return self.render(id, state);
                                }
                            }
                            self.load(ctx, id, false).await?;
                        }
                    }
                }
                _ => return Err("Unsupported Font Awesome operation".into()),
            }
            self.render(id, state)
        })
    }
}
fn valid_selection(value: &str, pro: bool) -> bool {
    (STYLES.iter().any(|(p, _)| *p == value) && (pro || ["fas", "fab"].contains(&value)))
        || (pro
            && value
                .strip_prefix("kit:")
                .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())))
}

#[cfg(test)]
mod tests;

fn copy_hud(action: &str) -> &'static str {
    match action {
        "copyIconName" => "Copied slug to clipboard!",
        "copyIconClasses" => "Copied classes to clipboard!",
        "copyAsSvg" => "Copied SVG to clipboard!",
        "copyIconGlyph" => "Copied glyph to clipboard!",
        _ => "Copied unicode to clipboard!",
    }
}
