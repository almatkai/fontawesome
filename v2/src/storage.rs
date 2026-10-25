// Only opaque remembered selection IDs are persisted, never OAuth/API tokens or SVGs.
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::Mutex,
};
use tezbar_extension_sdk::ExtensionContext;
static LANE: Mutex<()> = Mutex::new(());
fn path(ctx: &ExtensionContext) -> Result<PathBuf, String> {
    let parent = ctx.paths.data_dir.join("fontawesome");
    for p in [&ctx.paths.data_dir, &parent] {
        if let Ok(m) = std::fs::symlink_metadata(p) {
            if m.file_type().is_symlink() || !m.is_dir() {
                return Err("Invalid private storage directory".into());
            }
        }
    }
    let path = parent.join("storage.json");
    if let Ok(m) = std::fs::symlink_metadata(&path) {
        if !m.is_file() || m.file_type().is_symlink() || m.len() > 32768 {
            return Err("Invalid private selection storage".into());
        }
    }
    Ok(path)
}
fn read_inner(path: &std::path::Path) -> Result<Value, String> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(json!({})),
        Err(_) => return Err("Cannot read private selection storage".into()),
    };
    let mut bytes = Vec::new();
    file.take(32769)
        .read_to_end(&mut bytes)
        .map_err(|_| "Private storage read failed")?;
    if bytes.len() > 32768 {
        return Err("Private selection storage limit".into());
    }
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| "Corrupt private selection storage")?;
    let o = value.as_object().ok_or("Invalid selection store")?;
    if o.len() > 64
        || o.iter().any(|(k, v)| {
            k.len() != 64
                || !k.bytes().all(|b| b.is_ascii_hexdigit())
                || !v.as_str().is_some_and(|s| super::valid_selection(s, true))
        })
    {
        return Err("Invalid selection store contents".into());
    }
    Ok(value)
}
pub fn read(ctx: &ExtensionContext) -> Result<Value, String> {
    let _lane = LANE.lock().map_err(|_| "Storage unavailable")?;
    read_inner(&path(ctx)?)
}
pub fn save(ctx: &ExtensionContext, scope: &str, selection: &str) -> Result<(), String> {
    let _lane = LANE.lock().map_err(|_| "Storage unavailable")?;
    let path = path(ctx)?;
    let mut value = read_inner(&path)?;
    let o = value.as_object_mut().unwrap();
    if !o.contains_key(scope) && o.len() >= 64 {
        return Err("Selection store limit".into());
    }
    o.insert(scope.into(), json!(selection));
    std::fs::create_dir_all(path.parent().unwrap())
        .map_err(|_| "Private directory creation failed")?;
    let mut temp = tempfile::NamedTempFile::new_in(path.parent().unwrap())
        .map_err(|_| "Private storage staging failed")?;
    temp.write_all(
        serde_json::to_string(&value)
            .map_err(|_| "Storage encoding failed")?
            .as_bytes(),
    )
    .map_err(|_| "Storage write failed")?;
    temp.as_file()
        .sync_all()
        .map_err(|_| "Storage sync failed")?;
    temp.persist(&path)
        .map_err(|_| "Storage replacement failed")?;
    #[cfg(unix)]
    std::fs::File::open(path.parent().unwrap())
        .and_then(|f| f.sync_all())
        .map_err(|_| "Storage directory sync failed")?;
    Ok(())
}
