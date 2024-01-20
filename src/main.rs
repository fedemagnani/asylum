use asylum::{
    common,
    Asylum,
};

#[tokio::main]
async fn main() {
    let capacity = 10;
    let config_path = "Config.toml";
    let secret_path = "Secrets.toml";
    let log_level = "info";
    common::init_logger(Some(log_level));
    Asylum::print_asylum();
    let asylum = Asylum::new(
        Some(config_path),
        Some(secret_path),
        Some(capacity),
    )
    .await;
    asylum.start().await;
    // loop {
    //     println!("Hello, world!");
    //     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    // }
}