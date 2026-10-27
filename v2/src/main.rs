#[tokio::main(worker_threads = 2)]
async fn main() {
    if tezbar_extension_sdk::serve(
        fontawesome_v2::Adapter::new().expect("HTTPS client initialization"),
    )
    .await
    .is_err()
    {
        eprintln!("Font Awesome protocol terminated");
        std::process::exit(1);
    }
}
