use super::*;
use std::{path::Path, time::Duration};
use tezbar_extension_sdk::{ExtensionPaths, TestContext};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    process::{Child, ChildStdin, Command},
    task::{JoinHandle, JoinSet},
};
fn icon(name: &str) -> Value {
    json!({"id":name,"unicode":"1f600","svgs":[{"html":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\"><path d=\"M0 0L16 16\"/></svg>","familyStyle":{"prefix":"fas"}}]})
}
#[test]
fn original_oracle_parity() {
    let oracle: Value = serde_json::from_str(include_str!("../fixtures/oracle.json")).unwrap();
    let styles: Vec<_> = STYLES.iter().map(|(p, n)| json!([p, n])).collect();
    assert_eq!(json!(styles), oracle["styles"]);
    for case in oracle["copies"].as_array().unwrap() {
        let icon: Icon = serde_json::from_value(case["icon"].clone()).unwrap();
        assert_eq!(
            copy_text(&icon, case["action"].as_str().unwrap()).unwrap(),
            case["text"].as_str().unwrap()
        );
    }
    for case in oracle["queries"].as_array().unwrap() {
        let actual = search_body("hello world", case["prefix"].as_str().unwrap()).unwrap();
        let compact = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
        assert_eq!(
            compact(actual["query"].as_str().unwrap()),
            compact(case["body"].as_str().unwrap())
        );
    }
    for case in oracle["filters"].as_array().unwrap() {
        let kits: Vec<Kit> = serde_json::from_value(case["kits"].clone()).unwrap();
        let actual: Vec<_> = filtered_kits(&kits, case["filter"].as_str().unwrap())
            .iter()
            .map(|k| json!({"name":k.name,"token":k.token}))
            .collect();
        assert_eq!(json!(actual), case["result"]);
    }
    for case in oracle["kitNormalization"].as_array().unwrap() {
        let kits: Vec<Kit> = serde_json::from_value(case["kits"].clone()).unwrap();
        let actual =
            kit_icons(&kits, &kits[0].opaque_id(), case["query"].as_str().unwrap()).unwrap();
        let mut expected = case["result"].clone();
        for i in expected.as_array_mut().unwrap() {
            if i["unicode"].is_number() {
                i["unicode"] = json!(i["unicode"].to_string());
            }
        }
        assert_eq!(serde_json::to_value(actual).unwrap(), expected);
    }
    assert_eq!(glyph("d800"), "�"); // documented JSON/Rust surrogate adaptation
}
#[test]
fn encoded_query_and_sanitized_images() {
    let query = "\" } injected { \n\\";
    let text = search_body(query, "fasds").unwrap()["query"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(text.contains(&serde_json::to_string(query).unwrap()));
    assert!(text.contains("SHARP_DUOTONE"));
    let good = icon("safe")["svgs"][0]["html"].as_str().unwrap().to_owned();
    assert!(safe_image(&good)
        .unwrap()
        .starts_with("data:image/svg+xml;base64,"));
    for bad in [
        "<!DOCTYPE svg [<!ENTITY x SYSTEM 'file:///private'>]><svg>&x;</svg>",
        "<svg viewBox='0 0 1 1'><image href='https://example.invalid/leak'/></svg>",
        "<svg viewBox='0 0 1 1'><script>hi</script></svg>",
        "<svg viewBox='0 0 NaN 1'><path d='M0 0'/></svg>",
    ] {
        assert!(safe_image(bad).is_none());
    }
    assert!(!safe_image("<svg onload='evil' viewBox='0 0 1 1'><path d='M0 0' style='fill:url(https://example.invalid)'/></svg>").unwrap().contains("https"));
    for bad in [
        Value::Null,
        json!(true),
        json!({"ok":true,"error":"no"}),
        json!({"ok":true,"message":false}),
        json!({"ok":false}),
        json!({"ok":true,"unknown":1}),
    ] {
        assert!(!strict_ack(&bad));
    }
    assert!(strict_ack(&json!({"ok":true})));
    assert!(strict_ack(&json!({"ok":true,"message":"done"})));
}
struct Server {
    url: String,
    task: JoinHandle<()>,
    requests: Arc<Mutex<Vec<String>>>,
    certificate: Option<Vec<u8>>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn response<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    mut stream: S,
    requests: Arc<Mutex<Vec<String>>>,
) {
    let mut bytes = Vec::new();
    loop {
        let mut byte = [0];
        if stream.read_exact(&mut byte).await.is_err() {
            return;
        }
        bytes.push(byte[0]);
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
        if bytes.len() > 16384 {
            return;
        }
    }
    let headers = String::from_utf8_lossy(&bytes);
    let length = headers
        .lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length: ")
                .map(str::to_owned)
        })
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    if length > 8192 {
        return;
    }
    let start = bytes.len();
    bytes.resize(start + length, 0);
    if stream.read_exact(&mut bytes[start..]).await.is_err() {
        return;
    }
    let body = String::from_utf8_lossy(&bytes[start..]).to_string();
    let headers = String::from_utf8_lossy(&bytes[..start]);
    let token_req = headers.starts_with("POST /token ");
    requests.lock().unwrap().push(if token_req {
        "token".into()
    } else {
        body.clone()
    });
    if body.contains("slow") {
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    let mut status = "200 OK";
    let mut extra = "";
    let value = if token_req {
        if headers.contains("local-invalid") {
            status = "401 Unauthorized";
            json!({"error":"local"})
        } else {
            let scopes = if headers.contains("local-missing") {
                vec![]
            } else {
                vec!["kits_read", "domains_read", "svg_icons_pro"]
            };
            json!({"access_token":"local-access-only","expires_in":3600,"scopes":scopes,"token_type":"Bearer"})
        }
    } else if body.contains("GraphError") {
        json!({"errors":[{"message":"local"}],"data":{"search":[icon("partial-must-not-display")]}})
    } else if body.contains("redirect") {
        status = "302 Found";
        extra = "Location: https://example.invalid/blocked\r\n";
        json!({})
    } else if body.contains("rate-limit") {
        status = "429 Too Many Requests";
        json!({})
    } else if body.contains("long-reflected") {
        json!({"data":{"search":[icon("local\"pro-only")]}})
    } else if body.contains("reflected") {
        json!({"data":{"search":[icon("local-access-only")]}})
    } else if body.contains("KitIcons") {
        json!({"data":{"me":{"kits":[{"name":"Personal","token":"local-kit","iconUploads":[{"name":"custom-one","unicode":"e001","width":16,"height":16,"pathData":["M0 0L1 1"]}]}]}}})
    } else if body.contains("query Kits") {
        json!({"data":{"me":{"kits":[{"name":"Personal","token":"local-kit"}]}}})
    } else {
        let name = if body.contains("fresh") {
            "fresh"
        } else if body.contains("slow") {
            "slow"
        } else if body.contains("alpha") {
            "alpha"
        } else {
            "default-icon"
        };
        json!({"data":{"search":[icon(name)]}})
    };
    let mut data = value.to_string();
    if body.contains("oversized") {
        data = "x".repeat(2 * 1024 * 1024 + 1);
    }
    if body.contains("bad-json") {
        data = "{".into();
    }
    if body.contains("too-many") {
        data =
            json!({"data":{"search":(0..49).map(|i|icon(&format!("r{i}"))).collect::<Vec<_>>()}})
                .to_string();
    }
    let header=format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n{extra}Connection: close\r\n\r\n",data.len());
    let _ = stream.write_all(header.as_bytes()).await;
    let _ = stream.write_all(data.as_bytes()).await;
    let _ = stream.shutdown().await;
}
async fn server(tls: bool) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let r = requests.clone();
    let (acceptor, certificate) = if tls {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let der = cert.cert.der().to_vec();
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert.cert.der().clone()],
            rustls::pki_types::PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into()),
        )
        .unwrap();
        (
            Some(tokio_rustls::TlsAcceptor::from(Arc::new(config))),
            Some(der),
        )
    } else {
        (None, None)
    };
    let task = tokio::spawn(async move {
        let mut tasks = JoinSet::new();
        loop {
            tokio::select! {accepted=listener.accept()=>{let Ok((s,_))=accepted else{break};let r=r.clone();let acceptor=acceptor.clone();tasks.spawn(async move {if let Some(a)=acceptor{if let Ok(s)=a.accept(s).await{response(s,r).await;}}else{response(s,r).await;}});},_=tasks.join_next(),if !tasks.is_empty()=>{}}
        }
    });
    Server {
        url: format!(
            "{}://{}:{port}",
            if tls { "https" } else { "http" },
            if tls { "localhost" } else { "127.0.0.1" }
        ),
        task,
        requests,
        certificate,
    }
}
#[tokio::test]
async fn http_tls_cache_scope_errors() {
    let server = server(false).await;
    let api = Api::fixture(&server.url, None);
    let free = Preferences::parse(&json!({})).unwrap();
    let pro = Preferences::parse(&json!({"API_TOKEN":"local-pro-only","STYLE_PREFERENCE":"fat"}))
        .unwrap();
    let a = free.scope("profile-a");
    let b = pro.scope("profile-a");
    assert_ne!(a, b);
    assert_ne!(a, free.scope("profile-b"));
    let (icons, kits) = api.fetch(&free, &a, "alpha", "fas", false).await.unwrap();
    assert_eq!(icons[0].id, "alpha");
    assert!(kits.is_empty());
    let count = server.requests.lock().unwrap().len();
    api.fetch(&free, &a, "alpha", "fas", false).await.unwrap();
    assert_eq!(count, server.requests.lock().unwrap().len());
    api.fetch(&pro, &b, "alpha", "fat", false).await.unwrap();
    assert_eq!(
        server
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.as_str() == "token")
            .count(),
        2
    );
    let (_, kits) = api.fetch(&pro, &b, "alpha", "fat", false).await.unwrap();
    let (icons, _) = api
        .fetch(&pro, &b, "custom", &kits[0].opaque_id(), false)
        .await
        .unwrap();
    assert_eq!(icons[0].id, "custom-one");
    for text in [
        "GraphError",
        "redirect",
        "rate-limit",
        "bad-json",
        "too-many",
        "oversized",
        "reflected",
    ] {
        assert!(
            api.fetch(&free, &a, text, "fas", true).await.is_err(),
            "{text}"
        );
    }
    for token in ["local-invalid", "local-missing"] {
        let p = Preferences::parse(&json!({"API_TOKEN":token})).unwrap();
        assert!(api
            .fetch(&p, &p.scope("profile"), "", "fas", false)
            .await
            .is_err());
    }
    let quoted = Preferences::parse(&json!({"API_TOKEN":"local\"pro-only"})).unwrap();
    assert!(api
        .fetch(
            &quoted,
            &quoted.scope("profile"),
            "long-reflected",
            "fas",
            false
        )
        .await
        .is_err());
    let tls = self::server(true).await;
    let untrusted = Api::fixture(&tls.url, None);
    assert!(untrusted
        .fetch(&free, &a, "alpha", "fas", false)
        .await
        .is_err());
    let trusted = Api::fixture(&tls.url, tls.certificate.as_deref());
    assert_eq!(
        trusted
            .fetch(&free, &a, "alpha", "fas", false)
            .await
            .unwrap()
            .0[0]
            .id,
        "alpha"
    );
}
fn test_context(dir: &Path) -> TestContext {
    TestContext::new(
        ExtensionPaths {
            package_dir: dir.into(),
            data_dir: dir.join("data"),
            temp_dir: dir.join("temp"),
        },
        &["clipboard.write", "settings.open"],
    )
}
#[tokio::test]
async fn cancelled_open_drop_guard_and_corrupt_store() {
    let server = server(false).await;
    let adapter = Adapter {
        api: Api::fixture(&server.url, None),
        sessions: Arc::default(),
    };
    let temp = tempfile::tempdir().unwrap();
    let context = test_context(temp.path());
    let mut state = json!({});
    let payload = json!({"sessionId":"cancel-open","searchText":"slow","preferences":{}});
    assert!(tokio::time::timeout(
        Duration::from_millis(30),
        adapter.handle(
            &context.context,
            "index",
            "extension:run-command",
            &payload,
            &mut state
        )
    )
    .await
    .is_err());
    // TestContext has no dispose API; dropping the captured SDK task scope aborts its guard.
    drop(context);
    tokio::task::yield_now().await;
    assert!(adapter.lock().unwrap().is_empty());
    let ctx = test_context(temp.path());
    std::fs::create_dir_all(temp.path().join("data/fontawesome")).unwrap();
    std::fs::write(temp.path().join("data/fontawesome/storage.json"), b"{bad").unwrap();
    let mut state = json!({});
    let result = adapter
        .handle(
            &ctx.context,
            "index",
            "extension:run-command",
            &json!({"sessionId":"corrupt","preferences":{"REMEMBER_LAST_KIT":true}}),
            &mut state,
        )
        .await
        .unwrap();
    assert!(result["root"]["props"]["error"]
        .as_str()
        .unwrap()
        .contains("corrupt"));
    assert!(result["actions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["title"] == "Retry Font Awesome Request"));
}
struct Pipe {
    child: Child,
    ids: HashMap<String, String>,
    input: Option<ChildStdin>,
    output: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
}
impl Pipe {
    async fn start(server: &Server, dir: &Path) -> Self {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "tests::pipe_child", "--nocapture", "--quiet"])
            .env_clear()
            .env("FA_TEST_ENDPOINT", &server.url)
            .env("FA_TEST_QA", dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().unwrap()).lines();
        let mut p = Self {
            child,
            ids: HashMap::new(),
            input,
            output,
        };
        for _ in 0..8 {
            let line = p.line().await;
            if let Ok(v) = serde_json::from_str::<Value>(&line) {
                assert_eq!(v["type"], "hello");
                assert_eq!(v["sdkVersion"], "0.1.1");
                break;
            }
        }
        p.send(json!({"type":"request","id":"init","method":"initialize","params":{"protocol":1,"extensionId":"com.tezbar.native.fontawesome","platform":"macOS","packageDir":dir,"dataDir":dir.join("data"),"tempDir":dir.join("temp"),"grantedCapabilities":["clipboard.write","settings.open"]}})).await;
        assert_eq!(p.reply("init").await["result"]["ready"], true);
        p
    }
    async fn line(&mut self) -> String {
        tokio::time::timeout(Duration::from_secs(5), self.output.next_line())
            .await
            .expect("pipe timeout")
            .unwrap()
            .expect("pipe EOF")
    }
    async fn frame(&mut self) -> Value {
        let line = self.line().await;
        let mut v: Value =
            serde_json::from_str(&line).unwrap_or_else(|e| panic!("JSONL frame {line:?}: {e}"));
        if v["type"] == "response" {
            if let Some(label) = v["id"].as_str().and_then(|id| self.ids.get(id)) {
                v["id"] = json!(label);
            }
        }
        v
    }
    async fn send(&mut self, mut v: Value) {
        fn wire(label: &str) -> String {
            let d = digest(label);
            format!(
                "h:{}-{}-{}-{}-{}",
                &d[..8],
                &d[8..12],
                &d[12..16],
                &d[16..20],
                &d[20..32]
            )
        }
        if v["type"] == "request" {
            let label = v["id"].as_str().unwrap().to_owned();
            let id = wire(&label);
            self.ids.insert(id.clone(), label);
            v["id"] = json!(id);
        }
        if v["type"] == "cancel" {
            v["requestId"] = json!(wire(v["requestId"].as_str().unwrap()));
        }
        self.input
            .as_mut()
            .unwrap()
            .write_all(format!("{v}\n").as_bytes())
            .await
            .unwrap();
    }
    async fn reply(&mut self, id: &str) -> Value {
        loop {
            let v = self.frame().await;
            if v["type"] == "response" && v["id"] == id {
                return v;
            }
            assert_eq!(v["type"], "event", "unexpected frame: {v}");
        }
    }
    async fn open(&mut self, id: &str, sid: &str, prefs: Value, search: &str) -> Value {
        self.send(json!({"type":"request","id":id,"method":"session.open","params":{"sessionId":sid,"commandName":"index","preferences":prefs,"searchText":search}})).await;
        self.reply(id).await
    }
    async fn call(&mut self, id: &str, sid: &str, op: &str, payload: Value) {
        self.send(json!({"type":"request","id":id,"method":"session.call","params":{"sessionId":sid,"operation":op,"payload":payload}})).await;
    }
}
#[test]
fn pipe_child() {
    let Ok(endpoint) = std::env::var("FA_TEST_ENDPOINT") else {
        return;
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let adapter = Adapter {
            api: Api::fixture(&endpoint, None),
            sessions: Arc::default(),
        };
        let registry = adapter.sessions.clone();
        tezbar_extension_sdk::serve(adapter).await.unwrap();
        assert!(registry.lock().unwrap().is_empty());
    });
    std::process::exit(0);
}
fn first_action(v: &Value) -> String {
    v["result"]["root"]["children"][0]["props"]["actionIds"][0]
        .as_str()
        .unwrap()
        .into()
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_os_pipe_search_cancel_stale_retry_effect_dispose_eof() {
    let server = server(false).await;
    let temp = tempfile::tempdir().unwrap();
    let mut p = Pipe::start(&server, temp.path()).await;
    let opened = p.open("open", "s", json!({}), "alpha").await;
    assert_eq!(opened["result"]["root"]["type"], "Grid");
    let stale = first_action(&opened);
    // Latest query wins through the actual SDK reader, including A/B/A and cancelled search.
    p.call(
        "slow",
        "s",
        "extension:search-text-changed",
        json!({"searchText":"slow"}),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(30)).await;
    p.call(
        "fresh",
        "s",
        "extension:search-text-changed",
        json!({"searchText":"fresh"}),
    )
    .await;
    let mut responses = HashMap::new();
    for _ in 0..2 {
        let v = p.frame().await;
        responses.insert(v["id"].as_str().unwrap().to_owned(), v);
    }
    assert!(responses["slow"]["error"].is_object());
    assert_eq!(
        responses["fresh"]["result"]["root"]["children"][0]["props"]["title"],
        "fresh"
    );
    p.call(
        "stale",
        "s",
        "extension:invoke-action",
        json!({"actionId":stale}),
    )
    .await;
    assert!(p.reply("stale").await["error"].is_object());
    p.call(
        "alpha",
        "s",
        "extension:search-text-changed",
        json!({"searchText":"alpha"}),
    )
    .await;
    let current = p.reply("alpha").await;
    let copy = first_action(&current);
    p.call(
        "copy",
        "s",
        "extension:invoke-action",
        json!({"actionId":copy}),
    )
    .await;
    let reverse = p.frame().await;
    assert_eq!(reverse["method"], "host.clipboard.write");
    assert_eq!(reverse["params"], json!({"text":"alpha"}));
    // Bad ACK is uncertain, never successful and never silently replayed.
    p.send(
        json!({"type":"response","id":reverse["id"],"result":{"ok":true,"error":"contradiction"}}),
    )
    .await;
    let result = p.reply("copy").await;
    assert!(result["result"]["root"]["props"]["error"]
        .as_str()
        .unwrap()
        .contains("may have completed"));
    let retry = result["result"]["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["title"].as_str().unwrap().starts_with("Retry Prepared"))
        .unwrap()["id"]
        .clone();
    p.call(
        "retry-effect",
        "s",
        "extension:invoke-action",
        json!({"actionId":retry}),
    )
    .await;
    let reverse = p.frame().await;
    assert_eq!(reverse["params"], json!({"text":"alpha"}));
    p.send(json!({"type":"response","id":reverse["id"],"result":{"ok":true,"message":"copied"}}))
        .await;
    assert!(p.reply("retry-effect").await["result"].is_object());
    p.call(
        "cancelled",
        "s",
        "extension:search-text-changed",
        json!({"searchText":"slow-cancel"}),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    p.send(json!({"type":"cancel","requestId":"cancelled","reason":"fixture"}))
        .await;
    assert!(p.reply("cancelled").await["error"].is_object());
    p.call("refresh", "s", "extension:refresh-session", json!({}))
        .await;
    let v = p.reply("refresh").await;
    assert!(v["result"]["root"]["children"]
        .as_array()
        .unwrap()
        .iter()
        .all(|n| n["type"] != "Grid.Item"));
    p.call(
        "fail",
        "s",
        "extension:search-text-changed",
        json!({"searchText":"GraphError"}),
    )
    .await;
    let fail = p.reply("fail").await;
    let retry = fail["result"]["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["title"] == "Retry Font Awesome Request")
        .unwrap()["id"]
        .clone();
    p.call(
        "retry",
        "s",
        "extension:invoke-action",
        json!({"actionId":retry}),
    )
    .await;
    assert!(p.reply("retry").await["result"]["root"]["props"]["error"].is_string());
    p.send(json!({"type":"request","id":"dispose","method":"session.dispose","params":{"sessionId":"s"}})).await;
    assert!(p.reply("dispose").await["result"].is_object());
    p.call(
        "late",
        "s",
        "extension:invoke-action",
        json!({"actionId":copy}),
    )
    .await;
    assert!(p.reply("late").await["error"].is_object());
    drop(p.input.take());
    assert!(tokio::time::timeout(Duration::from_secs(3), p.child.wait())
        .await
        .unwrap()
        .unwrap()
        .success());
}
#[tokio::test]
async fn expiry_and_concurrent_scopes() {
    let server = server(false).await;
    let api = Api::fixture(&server.url, None);
    let prefs = Preferences::parse(&json!({})).unwrap();
    let scope = prefs.scope("expiry-profile");
    api.fetch(&prefs, &scope, "alpha", "fas", false)
        .await
        .unwrap();
    api.expire_fixture(&scope).await;
    api.fetch(&prefs, &scope, "alpha", "fas", false)
        .await
        .unwrap();
    assert_eq!(
        server
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.as_str() == "token")
            .count(),
        2
    );
    let (a, b, c) = tokio::join!(
        api.fetch(&prefs, "local-scope-a", "slow-a", "fas", false),
        api.fetch(&prefs, "local-scope-b", "slow-b", "fas", false),
        api.fetch(&prefs, "local-scope-c", "slow-c", "fas", false)
    );
    assert!(a.is_ok() && b.is_ok() && c.is_ok());
}
#[test]
fn preferences_and_private_storage_bounds() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = test_context(temp.path());
    let free = Preferences::parse(&json!({"STYLE_PREFERENCE":"fat"})).unwrap();
    assert_eq!(free.style, "fas");
    assert!(!free.pro);
    let pro = Preferences::parse(&json!({"API_TOKEN":"local-pro-only","STYLE_PREFERENCE":"fat"}))
        .unwrap();
    assert_eq!(pro.style, "fat");
    assert_ne!(free.scope("profile"), pro.scope("profile"));
    for prefs in [
        json!({"API_TOKEN":"\n"}),
        json!({"API_TOKEN":true}),
        json!({"API_TOKEN":"local","STYLE_PREFERENCE":"invented"}),
        json!({"REMEMBER_LAST_KIT":"true"}),
    ] {
        assert!(Preferences::parse(&prefs).is_err());
    }
    assert_eq!(storage::read(&ctx.context).unwrap(), json!({}));
    storage::save(&ctx.context, &free.scope("profile"), "fab").unwrap();
    storage::save(&ctx.context, &pro.scope("profile"), "fat").unwrap();
    let store = storage::read(&ctx.context).unwrap();
    assert_eq!(store[free.scope("profile")], "fab");
    assert_eq!(store[pro.scope("profile")], "fat");
    let path = temp.path().join("data/fontawesome/storage.json");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("local-pro-only"));
    assert!(!text.contains(api::FREE_TOKEN));
    std::fs::write(&path, "x".repeat(32769)).unwrap();
    assert!(storage::read(&ctx.context).is_err());
    assert!(storage::save(&ctx.context, &free.scope("profile"), "fas").is_err());
    #[cfg(unix)]
    {
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(temp.path().join("missing"), &path).unwrap();
        assert!(storage::read(&ctx.context).is_err());
    }
}
#[tokio::test]
async fn style_kit_order_remember_and_staged_revision() {
    let server = server(false).await;
    let adapter = Adapter {
        api: Api::fixture(&server.url, None),
        sessions: Arc::default(),
    };
    let temp = tempfile::tempdir().unwrap();
    let ctx = test_context(temp.path());
    let mut state = json!({});
    let prefs = json!({"API_TOKEN":"local-pro-only","PRIMARY_ACTION":"copyAsSvg","STYLE_PREFERENCE":"fat","REMEMBER_LAST_KIT":true});
    let payload = json!({"sessionId":"styles","preferences":prefs});
    let result = adapter
        .handle(
            &ctx.context,
            "index",
            "extension:run-command",
            &payload,
            &mut state,
        )
        .await
        .unwrap();
    let rows = result["root"]["children"].as_array().unwrap();
    let ids = rows[0]["props"]["actionIds"].as_array().unwrap();
    let first = result["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == ids[0])
        .unwrap();
    assert_eq!(first["title"], "Copy as SVG");
    let accessory = &result["root"]["props"]["searchBarAccessory"];
    assert_eq!(accessory["props"]["value"], "fat");
    assert_eq!(accessory["children"].as_array().unwrap().len(), 18);
    let selected = accessory["children"][17]["props"]["value"].clone();
    assert!(!result.to_string().contains("local-kit"));
    assert!(!result.to_string().contains("local-pro-only"));
    let changed=adapter.handle(&ctx.context,"index","extension:invoke-action",&json!({"sessionId":"styles","actionId":accessory["props"]["actionId"],"formValues":{"value":selected}}),&mut state).await.unwrap();
    assert_eq!(
        changed["root"]["children"][0]["props"]["title"],
        "custom-one"
    );
    assert_eq!(
        storage::read(&ctx.context).unwrap()[Preferences::parse(&prefs)
            .unwrap()
            .scope(&ctx.context.paths.data_dir.to_string_lossy())],
        selected
    );
    // Simulate SDK discarding a staged successful search response. Its intents must not authorize an effect.
    let committed = state.clone();
    let pending = adapter
        .handle(
            &ctx.context,
            "index",
            "extension:search-text-changed",
            &json!({"sessionId":"styles","searchText":"custom"}),
            &mut state,
        )
        .await
        .unwrap();
    state = committed;
    let stale = pending["root"]["children"][0]["props"]["actionIds"][0].clone();
    assert!(adapter
        .handle(
            &ctx.context,
            "index",
            "extension:invoke-action",
            &json!({"sessionId":"styles","actionId":stale}),
            &mut state
        )
        .await
        .is_err());
    assert!(ctx.calls().is_empty());
    let refresh = adapter
        .handle(
            &ctx.context,
            "index",
            "extension:refresh-session",
            &json!({"sessionId":"styles"}),
            &mut state,
        )
        .await
        .unwrap();
    assert!(refresh["root"]["children"]
        .as_array()
        .unwrap()
        .iter()
        .all(|n| n["type"] != "Grid.Item"));
    // A deterministic real adapter view feeds the pinned renderer contract probe.
    std::fs::create_dir_all("qa").unwrap();
    std::fs::write(
        "qa/renderer-view.json",
        serde_json::to_vec(&result).unwrap(),
    )
    .unwrap();
    adapter
        .handle(
            &ctx.context,
            "index",
            "extension:dispose-session",
            &json!({"sessionId":"styles"}),
            &mut state,
        )
        .await
        .unwrap();
    assert!(adapter.lock().unwrap().is_empty());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pipe_cancelled_open_and_uncertain_effect_no_replay() {
    let server = server(false).await;
    let temp = tempfile::tempdir().unwrap();
    let mut p = Pipe::start(&server, temp.path()).await;
    p.send(json!({"type":"request","id":"abandoned","method":"session.open","params":{"sessionId":"abandoned-session","commandName":"index","preferences":{},"searchText":"slow"}})).await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    p.send(json!({"type":"cancel","requestId":"abandoned","reason":"cancel open"}))
        .await;
    assert!(p.reply("abandoned").await["error"].is_object());
    p.call(
        "abandoned-refresh",
        "abandoned-session",
        "extension:refresh-session",
        json!({}),
    )
    .await;
    assert!(p.reply("abandoned-refresh").await["error"].is_object());
    let opened = p.open("open-uncertain", "effect", json!({}), "alpha").await;
    let copy = first_action(&opened);
    p.call(
        "effect-cancel",
        "effect",
        "extension:invoke-action",
        json!({"actionId":copy}),
    )
    .await;
    let reverse = p.frame().await;
    assert_eq!(reverse["method"], "host.clipboard.write");
    p.send(json!({"type":"cancel","requestId":"effect-cancel","reason":"host uncertain"}))
        .await;
    let mut saw_cancel = false;
    let mut saw_error = false;
    for _ in 0..2 {
        let frame = p.frame().await;
        if frame["type"] == "cancel" {
            assert_eq!(frame["requestId"], reverse["id"]);
            saw_cancel = true;
        } else {
            assert_eq!(frame["id"], "effect-cancel");
            assert!(frame["error"].is_object());
            saw_error = true;
        }
    }
    assert!(saw_cancel && saw_error);
    p.send(json!({"type":"response","id":reverse["id"],"result":{"ok":true}}))
        .await;
    p.call(
        "uncertain-refresh",
        "effect",
        "extension:refresh-session",
        json!({}),
    )
    .await;
    let refresh = p.reply("uncertain-refresh").await;
    assert!(refresh["result"]["root"]["props"]["error"]
        .as_str()
        .unwrap()
        .contains("may have completed"));
    assert!(refresh["result"]["actions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["title"].as_str().unwrap().starts_with("Retry Prepared")));
    // No auto-replay reverse request occurs during refresh or after a late ACK.
    drop(p.input.take());
    assert!(tokio::time::timeout(Duration::from_secs(3), p.child.wait())
        .await
        .unwrap()
        .unwrap()
        .success());
}
