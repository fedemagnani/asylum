use arkham::{ArkhamEntity, ArkhamTransaction};
use core::time;
use crossbeam_channel::{bounded, unbounded}; // We couldn't use crossbeam_channel since we need to broadcast messages (sending messages via crossbeam removes them from the channell)
use log::{debug, error, info, warn};
use serde_json::error;
use std::{sync::Arc, time::Duration};
use tokio::sync::broadcast;
use tokio_postgres::types::ToSql;
pub mod arkham;
pub mod common;
pub mod postgres;
// pub mod webbb;
pub mod webserver;
pub struct Asylum {
    // mpmc_sender: crossbeam_channel::Sender<common::AsylumMessage>, //probbaly not needed
    // mpmc_receiver: crossbeam_channel::Receiver<common::AsylumMessage>,
    broadcast_sender: broadcast::Sender<common::AsylumMessage>,
    config: Arc<serde_json::Value>, //this will never be changed, no reason to use a mutex
    arkham_client: arkham::MyArkham,
    postgres_client: Arc<tokio::sync::Mutex<postgres::MyPostgreSQL>>,
    task_counter: Arc<tokio::sync::Mutex<usize>>,
    config_path: Option<String>,
    secret_path: Option<String>,
}
impl Asylum {
    pub async fn new(
        config_path: Option<&str>,
        secret_path: Option<&str>,
        capacity: Option<usize>,
    ) -> Asylum {
        let config_path: String = config_path.unwrap_or("Config.toml").to_owned();
        let secret_path = secret_path.unwrap_or("Secrets.toml");
        let capacity = capacity.unwrap_or(100);
        let config = common::get_config(&config_path);
        let config = Arc::new(config);

        // let (sender, receiver) = unbounded::<common::AsylumMessage>();
        let (broadcast_sender, _) = broadcast::channel(capacity);
        let arkham_client = arkham::MyArkham::new(Some(secret_path.to_owned()));
        let postgres_client =
            postgres::MyPostgreSQL::new(Some(&secret_path), Some(&config_path))
                .await;
        let postgres_client = Arc::new(tokio::sync::Mutex::new(postgres_client));
        Asylum {
            // mpmc_sender: sender,
            // mpmc_receiver: receiver,
            broadcast_sender,
            config,
            arkham_client,
            postgres_client,
            task_counter: Arc::new(tokio::sync::Mutex::new(0)),
            config_path: Some(config_path),
            secret_path: Some(secret_path.to_owned()),
        }
    }
    pub fn print_asylum() {
        println!(" _______  _______           _                 _______ ");
        println!("(  ___  )(  ____ \\|\\     /|( \\      |\\     /|(       )");
        println!("| (   ) || (    \\/( \\   / )| (      | )   ( || () () |");
        println!("| (___) || (_____  \\ (_) / | |      | |   | || || || |");
        println!("|  ___  |(_____  )  \\   /  | |      | |   | || |(_)| |");
        println!("| (   ) |      ) |   ) (   | |      | |   | || |   | |");
        println!("| )   ( |/\\____) |   | |   | (____/\\| (___) || )   ( |");
        println!("|/     \\|\\_______)   \\_/   (_______/(_______)|/     \\|");
    }
    pub async fn create_table_entities(
        postgres_client: &mut postgres::MyPostgreSQL,
    ) -> Result<(), common::AsylumError> {
        let table_name = "ENTITIES";
        let columns = "id VARCHAR(255) PRIMARY KEY, name VARCHAR(255), website VARCHAR(255), twitter VARCHAR(255)";
        postgres_client
            .create_table(table_name, columns)
            .await
            .map_err(|e| common::AsylumError::PostgresError(e))?;
        Ok(())
    }
    pub async fn update_table_entities(
        postgres_client: &mut postgres::MyPostgreSQL,
        entities: Vec<arkham::ArkhamEntity>,
    ) -> Result<(), common::AsylumError> {
        for entity in entities {
            let insert_command = format!(
            "INSERT INTO {} (id, name, website, twitter) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
            "ENTITIES"
        );
            let params: &[&(dyn ToSql + Sync)] = &[
                &entity.id.unwrap_or_default(),
                &entity.name.unwrap_or_default(),
                &entity.website.unwrap_or_default(),
                &entity.twitter.unwrap_or_default(),
            ];
            postgres_client
                .execute(insert_command.as_str(), params)
                .await
                .map_err(|e| common::AsylumError::PostgresError(e))?;
        }
        Ok(())
    }
    pub async fn create_table_transactions(
        postgres_client: &mut postgres::MyPostgreSQL,
    ) -> Result<(), common::AsylumError> {
        let table_name = "TRANSACTIONS";
        // Columns are going to be the keys of ArkhamTransaction
        let columns = vec![
            "hash VARCHAR(255) PRIMARY KEY",
            "from_address VARCHAR(255)",
            "from_entity_id VARCHAR(255)",
            "from_entity_name VARCHAR(255)",
            "from_entity_label VARCHAR(255)",
            "from_entity_type VARCHAR(255)",
            "from_entity_twitter VARCHAR(255)",
            "to_address VARCHAR(255)",
            "to_entity_id VARCHAR(255)",
            "to_entity_name VARCHAR(255)",
            "to_entity_label VARCHAR(255)",
            "to_entity_type VARCHAR(255)",
            "to_entity_twitter VARCHAR(255)",
            "token_address VARCHAR(255)",
            "chain VARCHAR(255)",
            "block_number VARCHAR(255)",
            "block_timestamp VARCHAR(255)",
            "block_hash VARCHAR(255)",
        ];
        let columns = columns.join(", ");
        postgres_client
            .create_table(table_name, columns.as_str())
            .await
            .map_err(|e| common::AsylumError::PostgresError(e))?;
        Ok(())
    }
    pub async fn update_table_transactions(
        postgres_client: &mut postgres::MyPostgreSQL,
        transactions: Vec<arkham::ArkhamTransaction>,
    ) -> Result<(), common::AsylumError> {
        for transaction in transactions {
            let insert_command = format!(
            "INSERT INTO {} (hash, from_address, from_entity_id, from_entity_name, from_entity_label, from_entity_type, from_entity_twitter, to_address, to_entity_id, to_entity_name, to_entity_label, to_entity_type, to_entity_twitter, token_address, chain, block_number, block_timestamp, block_hash) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18) ON CONFLICT DO NOTHING",
            "TRANSACTIONS"
        );
            let params: &[&(dyn ToSql + Sync)] = &[
                &transaction.hash.unwrap_or_default(),
                &transaction.from_address.unwrap_or_default(),
                &transaction.from_entity_id.unwrap_or_default(),
                &transaction.from_entity_name.unwrap_or_default(),
                &transaction.from_entity_label.unwrap_or_default(),
                &transaction.from_entity_type.unwrap_or_default(),
                &transaction.from_entity_twitter.unwrap_or_default(),
                &transaction.to_address.unwrap_or_default(),
                &transaction.to_entity_id.unwrap_or_default(),
                &transaction.to_entity_name.unwrap_or_default(),
                &transaction.to_entity_label.unwrap_or_default(),
                &transaction.to_entity_type.unwrap_or_default(),
                &transaction.to_entity_twitter.unwrap_or_default(),
                &transaction.token_address.unwrap_or_default(),
                &transaction.chain.unwrap_or_default(),
                &transaction
                    .block_number
                    .map(|num| num.to_string())
                    .unwrap_or_default(), //block number is natively i64
                &transaction.block_timestamp.unwrap_or_default(),
                &transaction.block_hash.unwrap_or_default(),
            ];
            postgres_client
                .execute(insert_command.as_str(), params)
                .await
                .map_err(|e| common::AsylumError::PostgresError(e))?;
        }
        Ok(())
    }
    pub fn create_table_thread(&self) -> tokio::task::JoinHandle<()> {
        debug!("Creating tables");
        let mut postgres_client = self.postgres_client.clone();
        let task_counter = self.task_counter.clone();
        let table_thread = tokio::spawn(async move {
            *task_counter.lock().await += 1;
            let mut postgres_client = postgres_client.lock().await;
            Asylum::create_table_entities(&mut postgres_client)
                .await
                .unwrap();
            Asylum::create_table_transactions(&mut postgres_client)
                .await
                .unwrap();
            *task_counter.lock().await -= 1;
        });
        return table_thread;
    }
    pub async fn transactions_manager_thread(&self) -> tokio::task::JoinHandle<()> {
        let sender = self.broadcast_sender.clone();
        let mut receiver = self.broadcast_sender.subscribe();
        // let mpmc_sender = self.mpmc_sender.clone();
        let task_counter = self.task_counter.clone();
        let config = self.config.clone();
        // let config = config.lock().await;
        let execution_config = &config["execution_config"];
        let delay_config = &config["delay"];

        let max_entities_per_thread = execution_config
            .get("max_entities_per_thread")
            .expect("Max entities per thread must be set in your config file")
            .as_u64()
            .expect("Max entities per thread must be an integer")
            as usize;

        let time_last = execution_config
            .get("time_last")
            .expect("Time last must be set in your config file")
            .as_str()
            .map(|s| s.to_owned());

        let limit = execution_config
            .get("limit")
            .expect("Limit must be set in your config file")
            .as_u64();

        let transaction_delay = match delay_config.get("transactions_delay") {
            Some(delay) => delay
                .as_i64()
                .expect("Transactions delay must be an integer"),
            None => {
                warn!("Transactions delay not found in config file, using default 5");
                5
            }
        };

        let arkham_client = self.arkham_client.clone();
        let transactions_manager_thread = tokio::spawn(async move {
            *task_counter.lock().await += 1;
            let mut entities_chunks: Vec<Vec<ArkhamEntity>> = Vec::new();
            loop {
                if let Ok(am) = receiver.try_recv() {
                    match am {
                        common::AsylumMessage::Entities(entities) => {
                            info!("Received  {:?} entites", entities.len());
                            let entities: Vec<ArkhamEntity> = entities.iter().cloned().collect(); // Clone each entity
                            let chunks: Vec<Vec<ArkhamEntity>> = entities
                                .chunks(max_entities_per_thread)
                                .map(|chunk| chunk.to_vec())
                                .collect();
                            entities_chunks = chunks;
                        }
                        _ => (),
                    }
                }

                for chunk in &entities_chunks {
                    info!(
                        "Fetching transactions for chunk of length {:?}",
                        chunk.len()
                    );
                    let transactions = arkham_client
                        .retrieve_transactions(chunk, &time_last, &limit)
                        .await;
                    match transactions {
                        Ok(transactions) => {
                            sender
                                .send(common::AsylumMessage::Transactions(transactions))
                                .unwrap();
                            // mpmc_sender
                            //     .send(common::AsylumMessage::Transactions(transactions))
                            //     .unwrap();
                        }
                        Err(e) => error!("Error in fetching transactions: {:?}", e),
                    }
                    // tokio::time::sleep(Duration::from_secs(transaction_delay as u64)).await;
                } // Use a reference here
            }
            // *task_counter.lock().await -= 1;
        });
        return transactions_manager_thread;
    }
    pub fn entities_thread(&self) -> tokio::task::JoinHandle<()> {
        // let sender_a = self.mpmc_sender.clone();
        let sender_a = self.broadcast_sender.clone();
        let config = self.config.clone();
        let arkham = self.arkham_client.clone();
        let task_counter = self.task_counter.clone();
        let arkham_entities_thread = tokio::spawn(async move {
            *task_counter.lock().await += 1;
            let mut past_entities = None;
            // let c = config.lock().await;
            let delays = config
                .get("delay")
                .expect("Delays must be set in your config file");
            let entities_delay = delays
                .get("entities_delay")
                .expect("Entities delay must be set in your config file");
            let entities_delay = entities_delay
                .as_i64()
                .expect("Entities delay must be an integer");
            let entities_delay = entities_delay as u64;
            loop {
                let entities = arkham.retrieve_enitites(50).await;
                match entities {
                    Ok(entities) => {
                        if let Some(past_entities) = &past_entities {
                            if entities == *past_entities {
                                // warn!("Entities are the same as before");
                                continue;
                            }
                        }
                        past_entities = Some(entities.clone());
                        sender_a
                            .send(common::AsylumMessage::Entities(entities))
                            .unwrap();
                    }
                    Err(e) => error!("Error: {:?}", e),
                }
                tokio::time::sleep(Duration::from_secs(entities_delay)).await;
            }
            *task_counter.lock().await -= 1;
        });
        return arkham_entities_thread;
    }
    pub fn postgres_thread(&self) -> tokio::task::JoinHandle<()> {
        // let receiver = self.mpmc_receiver.clone();
        let mut receiver = self.broadcast_sender.subscribe();
        let postgres_client = self.postgres_client.clone();
        let task_counter = self.task_counter.clone();
        let postgres_thread = tokio::spawn(async move {
            *task_counter.lock().await += 1;
            let mut postgres_client = postgres_client.lock().await;

            loop {
                if let Ok(am) = receiver.recv().await {
                    match am {
                        common::AsylumMessage::Entities(entities) => {
                            debug!("{:?}", entities.len());
                            match Asylum::update_table_entities(&mut postgres_client, entities)
                                .await
                            {
                                Ok(_) => info!("Entities updated"),
                                Err(e) => error!("Error updating entities: {:?}", e),
                            }
                        }
                        common::AsylumMessage::Transactions(transactions) => {
                            debug!("{:?}", transactions.len());
                            match Asylum::update_table_transactions(
                                &mut postgres_client,
                                transactions,
                            )
                            .await
                            {
                                Ok(_) => info!("Transactions updated"),
                                Err(e) => error!("Error updating transactions: {:?}", e),
                            }
                        }
                        // common::AsylumMessage::TerminateThread => break,
                        _ => (),
                    }
                }
            }
            // *task_counter.lock().await -= 1;
        });
        return postgres_thread;
    }
    pub fn tasks_monitor_thread(&self) -> tokio::task::JoinHandle<()> {
        let task_counter = self.task_counter.clone();
        let tasks_monitor_thread = tokio::spawn(async move {
            *task_counter.lock().await += 1;
            loop {
                info!("Number of tasks: {}", *task_counter.lock().await);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            // *task_counter.lock().await -= 1;
        });
        return tasks_monitor_thread;
    }
    pub fn web_server_thread(&self) -> tokio::task::JoinHandle<()> {
        let config = self.config.clone();
        let config_path: Option<String> = self.config_path.clone();
        let secret_path = self.secret_path.clone();
        let web_server_thread = tokio::spawn(async move {
            info!("Starting webserver");
            // let config = config.lock().await;
            let port = config["webserver"]["port"]
                .as_u64()
                .expect("Port must be set in your config file") as u16;
            let web = webserver::WebServer::new(port);
            web.start( secret_path.as_deref(), config_path.as_deref())
                .await;
            error!("Webserver stopped");

        });
        return web_server_thread;
    }
    pub async fn start(&self) {
        // At the strtup, we connect the client, create tables if they don't exist
        error!("FIX BUILDING OF SQL FILTERS");
        error!("WEB SERVER THREAD IS BLOCKING OTHER THREADS");
        let web_server_thread = self.web_server_thread();
        let create_table_thread = self.create_table_thread();
        let transactions_manager_thread = self.transactions_manager_thread();
        let postgres_thread = self.postgres_thread();
        let arkham_entities_thread = self.entities_thread();
        let tasks_monitor_thread = self.tasks_monitor_thread();
        let _ = tokio::join!(
            web_server_thread,
            create_table_thread,
            transactions_manager_thread,
            postgres_thread,
            arkham_entities_thread,
            tasks_monitor_thread
        );
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_asylum() {
    info!("Starting test");
    // TODO:
    // optimize the code (avoid re-inizialization of the postgresql client)
    // create arkham message sent from arkham thread to postgres thread
    // create channels and threads for periodic querying of data
    // abstract this structure fetch stuff -> write to db
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
}
