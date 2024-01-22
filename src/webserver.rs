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
    pub limit: Option<i64>,
    pub page: Option<i64>,
}

pub struct WebServer {
    port: u16,
    limit_db_query: usize,
}
impl WebServer {
    pub fn new(port: u16, limit_db_query: usize) -> WebServer {
        WebServer {
            port,
            limit_db_query,
        }
    }
    pub async fn transaction_handler(
        args: (Arc<MyPostgreSQL>, TransactionQueryParams, usize),
    ) -> Result<impl Reply, Rejection> {
        let (db_client, query_params, limit_db_query) = args;
        let mut sql_query = format!("SELECT * FROM transactions",);
        let mut conditions = vec![];
        let mut limit = limit_db_query as usize;
        match query_params.limit {
            Some(l) => {
                limit = limit_db_query.min(l as usize);
                sql_query = format!("{} LIMIT {}", sql_query, limit);
            }
            None => {
                sql_query = format!("{} LIMIT {}", sql_query, limit_db_query);
            }
        }

        if let Some(page) = query_params.page {
            let page = page.max(1) - 1;
            let offset = page * limit as i64;
            sql_query = format!("{} OFFSET {}", sql_query, offset);
        }
        if let Some(from) = query_params.from {
            conditions.push(format!(
                "from_address ILIKE '{}' OR from_entity_id ILIKE '{}' OR from_entity_name ILIKE '{}'",
                from, from, from
            ));
        }
        if let Some(to) = query_params.to {
            conditions.push(format!(
                "to_address ILIKE '{}' OR to_entity_id ILIKE '{}' OR to_entity_name ILIKE '{}'",
                to, to, to
            ));
        }
        if let Some(token) = query_params.token {
            conditions.push(format!("token_address ILIKE '{}'", token));
        }
        if let Some(chain) = query_params.chain {
            conditions.push(format!("chain ILIKE '{}'", chain));
            if let Some(block) = query_params.block {
                conditions.push(format!(
                    " AND block_number ILIKE '{}' OR block_hash ILIKE '{}'",
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
        info!("Querying transactions with query: {}", sql_query);
        let rows = db_client.query(sql_query.as_str()).await.map_err(|e| {
            error!("Error querying transactions: {}", e);
            warp::reject::reject()
        })?;
        let mut txs: Vec<ArkhamTransaction> = rows
            .into_iter()
            .map(|row| {
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
                    block_number: row
                        .get::<_, Option<String>>("block_number")
                        .and_then(|s| s.parse::<i64>().ok()),
                    block_timestamp: row.get("block_timestamp"),
                    block_hash: row.get("block_hash"),
                };
                tx
            })
            .collect();
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
        let rows = db_client
            .query("SELECT * FROM entities")
            .await
            .map_err(|e| {
                error!("Error querying entities: {}", e);
                warp::reject::reject()
            })?;
        let entities: Vec<ArkhamEntity> = rows
            .into_iter()
            .map(|row| {
                let entity = ArkhamEntity {
                    id: row.get("id"),
                    name: row.get("name"),
                    website: row.get("website"),
                    twitter: row.get("twitter"),
                    type_: None,
                };
                entity
            })
            .collect();
        Ok(warp::reply::json(&entities))
    }
    pub fn init_routes(
        &self,
        db_client: Arc<MyPostgreSQL>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        let spendable_db_client = db_client.clone();
        let spendable_limit_db_query = self.limit_db_query;
        let transaction_route = warp::path!("transactions")
            .and(warp::get())
            .and(warp::query::<TransactionQueryParams>())
            .map(move |query_params| {
                (
                    spendable_db_client.clone(),
                    query_params,
                    spendable_limit_db_query,
                )
            })
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
        info!(
            "Starting webserver on port {}, use postman to interact with it!",
            self.port
        );
        let client = MyPostgreSQL::new(secrets_path, config_path).await;
        let client = Arc::new(client);
        let warp_filter = self.init_routes(client);
        warp::serve(warp_filter)
            .run(([0, 0, 0, 0], self.port))
            .await;
    }
}
