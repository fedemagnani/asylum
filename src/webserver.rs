use std::sync::Arc;

// This webserver is an overkill since we can query the data directly from the frontend, but it is nice to have it as a separate service, so that we can perform also custom queries
use crate::arkham::ArkhamTransaction;
use crate::{arkham::ArkhamEntity, postgres::MyPostgreSQL};
use async_trait::async_trait;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr}; //deserialization
use thiserror::Error;
use warp::{Filter, Rejection, Reply};
#[derive(Error, Debug)]
pub enum WebserverError {
    #[error("Internal server error.")]
    InternalServerError,
}

#[derive(Debug, Deserialize)]
pub struct TransactionQueryParams {
    pub from: Option<String>,  //either entity_id or entity_name
    pub to: Option<String>,    //either entity_id or entity_name
    pub token: Option<String>, //token address
    pub chain: Option<String>,
    pub block: Option<String>, //can be either a block number or a block hash. Can be used only if chain is specified
}

pub struct WebServer{
    port: u16,
}
impl WebServer {
    pub fn new(port: u16) -> WebServer {
        WebServer {
            port
        }
    }
    pub async fn transaction_handler(
        args: (Arc<MyPostgreSQL>, TransactionQueryParams),
    ) -> Result<impl Reply, Rejection> {
        let (db_client, query_params) = args;
        let mut sql_query = format!("SELECT * FROM transactions",);
        let mut conditions = vec![];
        if let Some(from) = query_params.from {
            conditions.push(format!(
                "from_address = {} OR from_entity_id = {} OR from_entity_name = {}",
                from, from, from
            ));
        }
        if let Some(to) = query_params.to {
            conditions.push(format!(
                "to_address = {} OR to_entity_id = {} OR to_entity_name = {}",
                to, to, to
            ));
        }
        if let Some(token) = query_params.token {
            conditions.push(format!("token_address = {}", token));
        }
        if let Some(chain) = query_params.chain {
            conditions.push(format!("chain = {}", chain));
            if let Some(block) = query_params.block {
                conditions.push(format!(
                    "block_number = {} OR block_hash = {}",
                    block, block
                ));
            }
        }
        let mut additional = String::new();
        if conditions.len() > 0 {
            additional = " WHERE ".to_owned();
            // now we append other conditions
            for (i, condition) in conditions.iter().enumerate() {
                if i == 0 {
                    additional = format!("{}{}", additional, condition);
                } else {
                    additional = format!("{} AND {}", additional, condition);
                }
            }
        }
        sql_query = format!("{}{}", sql_query, additional);
        let rows = db_client.query(sql_query.as_str()).await.map_err(|e| {
            error!("Error querying transactions: {}", e);
            warp::reject::reject()
        })?;
        let mut txs: Vec<ArkhamTransaction> = rows.into_iter().map(|row| {
            let tx = ArkhamTransaction {
                hash: row.get("hash"),
                from_address: row.get("from_address"),
                from_entity_id: row.get("from_entity_id"),
                from_entity_name: row.get("from_entity_name"),
                from_entity_label: row.get("from_entity_label"),
                from_entity_type: row.get("from_entity_type"),
                from_entity_twitter: row.get("from_entity_twitter"),
                to_address: row.get("to_address"),
                to_entity_id: row.get("to_entity_id"),
                to_entity_name: row.get("to_entity_name"),
                to_entity_label: row.get("to_entity_label"),
                to_entity_type: row.get("to_entity_type"),
                to_entity_twitter: row.get("to_entity_twitter"),
                token_address: row.get("token_address"),
                chain: row.get("chain"),
                block_number: row.get::<_, Option<String>>("block_number").and_then(|s| s.parse::<i64>().ok()),
                block_timestamp: row.get("block_timestamp"),
                block_hash: row.get("block_hash"),
            };
            tx
        }).collect();
        // If the vector is too big we might want to return a subset of it
        info!("{:?}", txs[0]);
        info!("Found {} transactions, showing the first one", txs.len());
        Ok(warp::reply::json(&txs))
    }

    pub fn pong_handler() -> impl Reply {
        info!("Pong!");
        warp::reply::json(&"pong")
    }

    pub async fn entities_handler(db_client: Arc<MyPostgreSQL>) -> Result<impl Reply, Rejection> {
        let entities: Vec<ArkhamEntity> = vec![];
        Ok(warp::reply::json(&entities))
    }

    pub fn init_routes(
        db_client: Arc<MyPostgreSQL>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        let spendable_db_client = db_client.clone();
        let transaction_route = warp::path!("transactions")
            .and(warp::get())
            .and(warp::query::<TransactionQueryParams>())
            .map(move |query_params| (spendable_db_client.clone(), query_params))
            .and_then(Self::transaction_handler);
        let spendable_db_client = db_client.clone();
        let entities_route = warp::path!("entities")
            .and(warp::get())
            .map(move || spendable_db_client.clone())
            .and_then(Self::entities_handler);
        let pong_route = warp::path!("ping").and(warp::get()).map(Self::pong_handler);

        let warp_filter = transaction_route.or(entities_route).or(pong_route);
        warp_filter.with(warp::log("api")).boxed()
    }

    pub async fn start(&self, secrets_path: Option<&str>, config_path: Option<&str>) {
        info!("Starting webserver on port {}, use postman to interact with it!", self.port);
        let client = MyPostgreSQL::new(secrets_path, config_path).await;
        let client = Arc::new(client);
        let warp_filter = Self::init_routes(client);
        warp::serve(warp_filter).run(([0, 0, 0, 0], self.port)).await;
    }
}
