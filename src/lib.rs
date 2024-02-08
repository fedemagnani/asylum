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
            postgres::MyPostgreSQL::new(Some(&secret_path), Some(&config_path)).await;
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
    pub async fn create_table_unique_addresses(
        postgres_client: &mut postgres::MyPostgreSQL,
    ) -> Result<(), common::AsylumError> {
        let table_name = "UNIQUE_ADDRESSES";
        let columns = "name VARCHAR(255), address VARCHAR(255) , chain VARCHAR(255), PRIMARY KEY (name, address, chain)";
        postgres_client
            .create_table(table_name, columns)
            .await
            .map_err(|e| common::AsylumError::PostgresError(e))?;
        Ok(())
    }
    pub async fn update_table_unique_addresses(
        postgres_client: &mut postgres::MyPostgreSQL,
    ) -> Result<(), common::AsylumError> {
        //we bet on the existence of a table called entities and transactions
        let insert_command = format!(
            "INSERT INTO UNIQUE_ADDRESSES (name, address, chain)
            SELECT from_entity_name as name_entity, from_address, CASE 
                WHEN chain = 'avalanche' THEN 'avalanche'
                WHEN chain = 'base' THEN 'base'
                WHEN chain = 'arbitrum_one' THEN 'arbitrum'
                WHEN chain = 'ethereum' THEN 'eth'
                WHEN chain = 'polygon' THEN 'polygon'
                ELSE '/'
            END FROM transactions WHERE from_entity_name IN (SELECT name from entities)
            UNION ALL
            SELECT to_entity_name, to_address, CASE 
                WHEN chain = 'avalanche' THEN 'avalanche'
                WHEN chain = 'base' THEN 'base'
                WHEN chain = 'arbitrum_one' THEN 'arbitrum'
                WHEN chain = 'ethereum' THEN 'eth'
                WHEN chain = 'polygon' THEN 'polygon'
                ELSE '/'
            END FROM transactions WHERE to_entity_name IN (SELECT name from entities) ON CONFLICT (name, address, chain) DO NOTHING",
        );
        postgres_client
            .execute(&insert_command, &[])
            .await
            .map_err(|e| common::AsylumError::PostgresError(e))?;
        Ok(())
    }
    pub async fn create_table_portfolio_holdings(
        postgres_client: &mut postgres::MyPostgreSQL,
    ) -> Result<(), common::AsylumError> {
        let table_name = "PORTFOLIO_HOLDINGS";
        let columns = vec![
            "timestamp",
            "entity_name",
            "chain",
            "token_name",
            "token_symbol",
            "balance",
            "price",
            "balance_in_dollars",
        ];
        let columns = columns.join(" VARCHAR(255), ");
        let columns = format!("{} VARCHAR(255)", columns);
        postgres_client
            .create_table(table_name, columns.as_str())
            .await
            .map_err(|e| common::AsylumError::PostgresError(e))?;
        Ok(())
    }
    pub async fn update_table_portfolio_holdings(
        postgres_client: &mut postgres::MyPostgreSQL,
        portfolio_holdings: Vec<arkham::ArkhamTokenHolding>,
        timestamp_millis: i64,
    ) -> Result<(), common::AsylumError> {
        for holding in portfolio_holdings {
            let insert_command = format!(
            "INSERT INTO {} (timestamp, entity_name, chain, token_name, token_symbol, balance, price, balance_in_dollars) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING",
            "PORTFOLIO_HOLDINGS"
        );
            // let timestamp_millis = chrono::Utc::now().timestamp_millis();
            let params: &[&(dyn ToSql + Sync)] = &[
                &timestamp_millis.to_string(),
                &holding.entity_name.unwrap_or_default(),
                &holding.chain.unwrap_or_default(),
                &holding.token_name.unwrap_or_default(),
                &holding.token_symbol.unwrap_or_default(),
                &holding.token_balance.unwrap_or_default(),
                &holding.token_price.unwrap_or_default(),
                &holding.token_balance_usd.unwrap_or_default(),
            ];
            postgres_client
                .execute(insert_command.as_str(), params)
                .await
                .map_err(|e| common::AsylumError::PostgresError(e))?;
        }
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
            "token_name VARCHAR(255)",
            "token_symbol VARCHAR(255)",
            "token_decimals VARCHAR(255)",
            "amount VARCHAR(255)",
            "dollar_amount VARCHAR(255)",
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
            "INSERT INTO {} (hash, from_address, from_entity_id, from_entity_name, from_entity_label, from_entity_type, from_entity_twitter, to_address, to_entity_id, to_entity_name, to_entity_label, to_entity_type, to_entity_twitter, token_address, token_name, token_symbol, token_decimals, amount, dollar_amount, chain, block_number, block_timestamp, block_hash) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23) ON CONFLICT DO NOTHING",
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
                &transaction.token_name.unwrap_or_default(),
                &transaction.token_symbol.unwrap_or_default(),
                &transaction
                    .token_decimals
                    .map(|num| num.to_string())
                    .unwrap_or_default(),
                &transaction.amount.unwrap_or_default(),
                &transaction.dollar_amount.unwrap_or_default(),
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
    pub async fn delete_old_data(
        postgres_client: &mut postgres::MyPostgreSQL,
        drop_seconds_transactions: i64,
        drop_seconds_portfolio: i64,
    ) -> Result<(), common::AsylumError> {
        let drop_seconds_transactions = drop_seconds_transactions.to_string();
        let drop_seconds_portfolio = drop_seconds_portfolio.to_string();
        let delete_command = format!("DELETE FROM TRANSACTIONS WHERE TO_TIMESTAMP(block_timestamp, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') < NOW() - INTERVAL '{} SECONDS'", drop_seconds_transactions);
        postgres_client
            .execute(delete_command.as_str(), &[])
            .await
            .map_err(|e| common::AsylumError::PostgresError(e))?;
        let delete_command = format!("DELETE FROM PORTFOLIO_HOLDINGS WHERE TO_TIMESTAMP((timestamp::bigint) / 1000.0) < NOW() - INTERVAL '{} SECONDS'", drop_seconds_portfolio);
        postgres_client
            .execute(delete_command.as_str(), &[])
            .await
            .map_err(|e| common::AsylumError::PostgresError(e))?;
        Ok(())
    } // in the same operation we delete older transactions

    pub fn create_table_thread(&self) -> tokio::task::JoinHandle<()> {
        debug!("Creating tables");
        let postgres_client = self.postgres_client.clone();
        
        let table_thread = tokio::spawn(async move {
            
            let mut postgres_client = postgres_client.lock().await; //Safe lock system: postgres_client is locked initially to create all the tables (we wait until they are created), then it is locked in postgres_thread inifiedly to update the tables (no other thread needs to access it, so it is safe to lock it in this way)
            Asylum::create_table_entities(&mut postgres_client)
                .await
                .expect("Error creating table entities");
            Asylum::create_table_transactions(&mut postgres_client)
                .await
                .expect("Error creating table transactions");
            Asylum::create_table_unique_addresses(&mut postgres_client)
                .await
                .expect("Error creating table unique addresses");
            Asylum::create_table_portfolio_holdings(&mut postgres_client)
                .await
                .expect("Error creating table portfolio holdings");
            
        });
        return table_thread;
    }
    pub async fn transactions_manager_thread(&self) -> tokio::task::JoinHandle<()> {
        let sender = self.broadcast_sender.clone();
        let mut receiver = self.broadcast_sender.subscribe();
        // let mpmc_sender = self.mpmc_sender.clone();
        
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
            
            let mut entities_chunks: Vec<Vec<ArkhamEntity>> = Vec::new();
            loop {
                if let Ok(am) = receiver.try_recv() {
                    match am {
                        common::AsylumMessage::Entities(entities, _) => {
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
                                .expect("Error sending thread message with transactions");
                            // mpmc_sender
                            //     .send(common::AsylumMessage::Transactions(transactions))
                            //     .unwrap();
                        }
                        Err(e) => error!("Error in fetching transactions: {:?}", e),
                    }
                    tokio::time::sleep(Duration::from_secs(transaction_delay as u64)).await;
                } // Use a reference here
            }
            // 
        });
        return transactions_manager_thread;
    }
    pub fn entities_thread(&self) -> tokio::task::JoinHandle<()> {
        // let sender_a = self.mpmc_sender.clone();
        let sender_a = self.broadcast_sender.clone();
        let config = self.config.clone();
        let arkham = self.arkham_client.clone();
        
        let arkham_entities_thread = tokio::spawn(async move {
            
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
                let timestamp_millis = chrono::Utc::now().timestamp_millis();
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
                            .send(common::AsylumMessage::Entities(entities, timestamp_millis))
                            .expect("Error sending thread message with entities");
                    }
                    Err(e) => error!("Error: {:?}", e),
                }
                tokio::time::sleep(Duration::from_secs(entities_delay)).await;
            }
            
        });
        return arkham_entities_thread;
    }

    pub fn portfolio_holdings_thread(&self) -> tokio::task::JoinHandle<()> {
        let sender = self.broadcast_sender.clone();
        let mut receiver = self.broadcast_sender.subscribe();
        let arkham = self.arkham_client.clone();
        
        let config = self.config.clone();
        let portfolio_holdings_thread = tokio::spawn(async move {
            
            let delays = config
                .get("delay")
                .expect("Delays must be set in your config file");
            let portfolio_holdings_delay = delays
                .get("portfolio_holdings_delay")
                .expect("Portfolio holdings delay must be set in your config file");
            let portfolio_holdings_delay = portfolio_holdings_delay
                .as_i64()
                .expect("Portfolio holdings delay must be an integer");
            let portfolio_holdings_delay = portfolio_holdings_delay as u64;
            loop {
                let mut len = 0;
                if let Ok(holdings) = receiver.try_recv() {
                    match holdings {
                        common::AsylumMessage::Entities(entities, timestamp_millis) => {
                            len = entities.len();
                            let entities: Vec<ArkhamEntity> = entities.iter().cloned().collect(); // Clone each entity

                            for entity in entities {
                                let holdings = arkham.retrieve_portfolio(&entity).await;
                                match holdings {
                                    Ok(holdings) => {
                                        sender
                                            .send(common::AsylumMessage::PortfolioHoldings(holdings, timestamp_millis))
                                            .expect("Error sending thread message with portfolio holdings");
                                    }
                                    Err(e) => {
                                        error!("Error in fetching portfolio holdings: {:?}", e)
                                    }
                                }
                                tokio::time::sleep(Duration::from_secs(portfolio_holdings_delay))
                                    .await;
                            }
                        }
                        _ => (),
                    }
                    info!("Updated portfolio holdings for {:?} entities", len);
                }
            }
            
        });
        return portfolio_holdings_thread;
    }

    pub fn postgres_thread(&self) -> tokio::task::JoinHandle<()> {
        // let receiver = self.mpmc_receiver.clone();
        let mut receiver = self.broadcast_sender.subscribe();
        let postgres_client = self.postgres_client.clone();
        
        let config = self.config.clone();
        let execution_config = &config["execution_config"];
        let drop_seconds_transactions = execution_config
            .get("drop_seconds_transactions")
            .expect("Drop seconds transactions must be set in your config file")
            .as_i64()
            .expect("Drop seconds transactions must be an integer");
        let drop_seconds_portfolio = execution_config
            .get("drop_seconds_portfolio")
            .expect("Drop seconds portfolio must be set in your config file")
            .as_i64()
            .expect("Drop seconds portfolio must be an integer");
        let postgres_thread = tokio::spawn(async move {
            
            let mut postgres_client = postgres_client.lock().await;

            //maybe a bottleneck here -> this resource will keep postgres_client busy (locked) and so it will not be able to be used by other threads, so we need to remove old datas from here. We avoid to create a dedicated thread to not increase the queue in the broadcast channel, so we remove old data when we update entities (which we are going to update more frequently)
            loop {
                if let Ok(am) = receiver.recv().await {
                    match am {
                        common::AsylumMessage::Entities(entities, _) => {
                            debug!("{:?}", entities.len());
                            match Asylum::update_table_entities(&mut postgres_client, entities)
                                .await
                            {
                                Ok(_) => info!("Entities updated"),
                                Err(e) => error!("Error updating entities: {:?}", e),
                            }
                            match Asylum::delete_old_data(
                                &mut postgres_client,
                                drop_seconds_transactions,
                                drop_seconds_portfolio,
                            )
                            .await {
                                Ok(_) => info!("Old data deleted"),
                                Err(e) => error!("Error deleting old data: {:?}", e),
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
                                Ok(_) => {
                                    info!("Transactions updated");
                                    match Asylum::update_table_unique_addresses(
                                        &mut postgres_client,
                                    )
                                    .await
                                    {
                                        Ok(_) => info!("Unique addresses updated"),
                                        Err(e) => {
                                            error!("Error updating unique addresses: {:?}", e)
                                        }
                                    }
                                }
                                Err(e) => error!("Error updating transactions: {:?}", e),
                            }
                        }
                        common::AsylumMessage::PortfolioHoldings(holdings, timestamp_millis) => {
                            debug!("{:?}", holdings.len());
                            match Asylum::update_table_portfolio_holdings(
                                &mut postgres_client,
                                holdings,
                                timestamp_millis
                            )
                            .await
                            {
                                Ok(_) => info!("Portfolio holdings updated"),
                                Err(e) => error!("Error updating portfolio holdings: {:?}", e),
                            }
                        }
                        _ => (),
                    }
                }
            }
            // 
        });
        return postgres_thread;
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
            let limit_db_query = config["webserver"]["limit_db_query"]
                .as_u64()
                .expect("Limit db query must be set in your config file")
                as usize;
            let web = webserver::WebServer::new(port, limit_db_query);
            web.start(secret_path.as_deref(), config_path.as_deref())
                .await;
            error!("Webserver stopped");
        });
        return web_server_thread;
    }
    pub async fn start(&self) {
        // // At the strtup, we connect the client, create tables if they don't exist
        // let web_server_thread = self.web_server_thread();
        let create_table_thread = self.create_table_thread();
        let transactions_manager_thread = self.transactions_manager_thread();
        let postgres_thread = self.postgres_thread();
        let arkham_entities_thread = self.entities_thread();
        let portfolio_holdings_thread = self.portfolio_holdings_thread();
        let _ = tokio::join!(
            // web_server_thread,
            create_table_thread,
            transactions_manager_thread,
            postgres_thread,
            arkham_entities_thread,
            portfolio_holdings_thread
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
    let asylum = Asylum::new(Some(config_path), Some(secret_path), Some(capacity)).await;
    asylum.start().await;
}
