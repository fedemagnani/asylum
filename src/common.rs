use shuttle_secrets;
use serde_json::{Value};
use env_logger::Env;
use crate::arkham;

#[derive(Debug)]
pub enum AsylumError {
    ArkhamError(String),
    PostgresError(postgres::Error),
    ReqwestError(reqwest::Error),
    SerdeError(serde_json::Error),
}
#[derive(Clone, Debug)]
pub enum AsylumMessage{
    Entities(Vec<arkham::ArkhamEntity>),
    Transactions(Vec<arkham::ArkhamTransaction>),
    TerminateThread,
    // QueryInstruction()
}

pub fn init_logger(log_level: Option<&str>) {
    let log_level = log_level.unwrap_or("info");
    env_logger::Builder::from_env(Env::default().default_filter_or(log_level)).init();
}

pub fn get_secrets(path:&str)-> shuttle_secrets::SecretStore {
        // Read the file into a String
    let contents = std::fs::read_to_string(path)
        .expect("Failed to read Secrets.toml");
    let contents = format!("{}{}", "[secrets]\n", contents);
    toml::from_str(&contents)
        .expect("Failed to parse Secrets.toml")
}

pub fn get_config(path:&str)-> Value {
    // Read the file into a String
    let contents = std::fs::read_to_string(path)
        .expect("Failed to read Config.toml");
    // let contents = format!("{}{}", "[config]\n", contents);
    toml::from_str(&contents)
        .expect("Failed to parse Config.toml")
}
