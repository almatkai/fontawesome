#!/usr/bin/env python3
"""Produce an unapplied patch against the pinned host; never access its worktree."""
import pathlib,difflib,re
root=pathlib.Path(__file__).resolve().parents[1]
base=root/'fixtures/host'; out=root/'qa/patched-host';out.mkdir(parents=True,exist_ok=True)
path='src-tauri/src/native_core/extensions/registry.rs'
s=(base/path).read_text(); original=s
s=s.replace('    path: PathBuf,\n','    path: PathBuf,\n    resource_dir: PathBuf,\n',1)
s=s.replace('            let mut manifest = if native {','            let mut resource_dir = path.clone();\n            let mut manifest = if native {',1)
s=s.replace('Ok(package) => package.manifest,','Ok(package) => { resource_dir = package.dir; package.manifest },',1)
s=s.replace('                path,\n                manifest,','                path,\n                resource_dir,\n                manifest,',1)
s=s.replace('        path,\n        manifest: Value::Null,','        resource_dir: path.clone(),\n        path,\n        manifest: Value::Null,',1)
s=s.replace('extension.path.join(relative)','extension.resource_dir.join(relative)').replace('extension.path.join("assets").join(relative)','extension.resource_dir.join("assets").join(relative)')
helper='''// Only exact aliases from a verified installed active manifest can select a process.
fn declares_replacement(manifest: &Value, requested: &str, command: &str) -> bool {
    manifest["id"].as_str().is_some_and(|id| id.starts_with("com.tezbar.native."))
        && manifest["healthError"].is_null()
        && manifest["runtime"] == "rust"
        && compatible(manifest, false)
        && manifest["replaces"].as_array().is_some_and(|aliases| aliases.iter().any(|alias| alias.as_str() == Some(requested)))
        && manifest["commands"].as_array().is_some_and(|commands| commands.iter().any(|c| c["name"] == command && c["runtime"] == "rust" && compatible(c, true)))
}
pub(super) fn replacement_id(ctx: &CoreContext, requested: &str, command: &str) -> Option<String> {
    let mut matches = collect(ctx).into_iter().filter(|e| declares_replacement(&e.manifest, requested, command));
    let selected = matches.next()?.id;
    // Ambiguous declarations never silently select a different package.
    if matches.next().is_some() { None } else { Some(selected) }
}

'''
s=s.replace('/// Native implementations use the same discovery',helper+'/// Native implementations use the same discovery',1)
s=s.replace('    let native = if id.starts_with("raycast.") {','    let native = if let Some(canonical) = replacement_id(ctx, id, command) {\n        installed(ctx, &canonical).ok()\n    } else if id.starts_with("raycast.") {',1)
tests='''
#[cfg(test)]
mod external_package_resolution_tests {
    use super::*;
    #[test]
    fn exact_verified_replacement_and_command_only() {
        let m = json!({"id":"com.tezbar.native.example", "runtime":"rust", "platforms":["macOS","Windows","Linux"],"replaces":["com.tezbar.example", "raycast.example"],"commands":[{"name":"index","runtime":"rust","platforms":["macOS","Windows","Linux"]}]});
        assert!(declares_replacement(&m, "com.tezbar.example", "index"));
        assert!(declares_replacement(&m, "raycast.example", "index"));
        assert!(!declares_replacement(&m, "example", "index"));
        assert!(!declares_replacement(&m, "com.tezbar.example-other", "index"));
        assert!(!declares_replacement(&m, "com.tezbar.example", "other"));
        let mut damaged=m.clone(); damaged["healthError"]=json!("tampered");
        assert!(!declares_replacement(&damaged,"com.tezbar.example","index"));
        let mut wrong=m.clone();wrong["commands"][0]["runtime"]=json!("javascript");
        assert!(!declares_replacement(&wrong,"com.tezbar.example","index"));
        wrong=m.clone(); wrong["commands"][0]["platforms"]=json!(["unsupported"]);
        assert!(!declares_replacement(&wrong,"com.tezbar.example","index"));
    }
    #[test]
    fn active_resources_keep_canonical_preferences_root() {
        let qa=tempfile::tempdir().unwrap(); let canonical=qa.path().join("packages/com.tezbar.native.example");
        let active=canonical.join("versions/2.0.0"); std::fs::create_dir_all(active.join("resources")).unwrap();
        std::fs::write(active.join("resources/icon.png"),b"pinned icon").unwrap();
        let e=Installed{id:"com.tezbar.native.example".into(),slug:"example".into(),path:canonical.clone(),resource_dir:active.clone(),manifest:json!({"icon":"resources/icon.png"}),data_dir:qa.path().into()};
        assert_eq!(icon_path(&e),Some(active.join("resources/icon.png").to_string_lossy().into()));
        assert_eq!(e.path,canonical); assert_ne!(e.path,e.resource_dir);
        assert_eq!(command_icon_path(&e,&json!({"icon":"resources/icon.png"})),icon_path(&e));
        assert!(command_icon_path(&e,&json!({"icon":"../outside.png"})).is_none());
    }
}
'''
s+=tests
p=out/path;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(s)
diff=''.join(difflib.unified_diff(original.splitlines(True),s.splitlines(True),fromfile='a/'+path,tofile='b/'+path))
path2='src-tauri/src/native_core/extensions/native.rs'; original2=(base/path2).read_text(); n=original2
n=n.replace('if requested.starts_with("com.tezbar.native.") || v2_runtime::slug(requested).is_some()', 'if requested.starts_with("com.tezbar.native.") || v2_runtime::slug(requested).is_some()\n                || registry::replacement_id(ctx, requested, body["commandName"].as_str().unwrap_or("")).is_some()',1)
n=n.replace('        } else {\n            v2_runtime::canonical_id(v2_runtime::slug(requested).ok_or("Unknown extension")?)','        } else if let Some(canonical) = registry::replacement_id(ctx, requested, command) {\n            canonical\n        } else {\n            v2_runtime::canonical_id(v2_runtime::slug(requested).ok_or("Unknown extension")?)',1)
p=out/path2;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(n)
diff+=''.join(difflib.unified_diff(original2.splitlines(True),n.splitlines(True),fromfile='a/'+path2,tofile='b/'+path2))
(root/'integration').mkdir(exist_ok=True);(root/'integration/0001-active-resources-exact-replacements.patch').write_text(diff)
# Compile the actual patched pure functions and same regression tests, without the app.
def function(text,name):
 start=re.search(r'fn '+name+r'(?:<[^>]+>)?\(',text).start(); brace=text.index('{',start); depth=1;i=brace+1
 while depth:
  if text[i]=='{':depth+=1
  elif text[i]=='}':depth-=1
  i+=1
 return text[start:i]
harness='use serde_json::{json,Value}; use std::path::{Path,PathBuf,Component};\nstruct Installed {id:String,slug:String,path:PathBuf,resource_dir:PathBuf,manifest:Value,data_dir:PathBuf}\n'
for f in ['platform','compatible','fallback','declares_replacement','icon_path','command_icon_path']:harness+=function(s,f)+'\n'
harness+=tests
(root/'qa/host-registry-contract.rs').write_text(harness)
print('Unapplied host patch and extracted regression harness generated from pinned files.')
