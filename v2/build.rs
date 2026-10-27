// Compile the original public free credential without logging or duplicating it in generated assets.
fn main() {
    let source = "fixtures/source/src/hooks/usePreferences.ts";
    println!("cargo:rerun-if-changed={source}");
    let text = std::fs::read_to_string(source).expect("pinned preferences source");
    let token = text
        .split_once("API_TOKEN = \"")
        .expect("original free fallback")
        .1
        .split('"')
        .next()
        .unwrap();
    let output = format!("pub const FREE_TOKEN: &str = {:?};", token);
    std::fs::write(
        std::path::Path::new(&std::env::var_os("OUT_DIR").unwrap()).join("free.rs"),
        output,
    )
    .unwrap();
}
