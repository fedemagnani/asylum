use crate::common::{get_secrets, init_logger, AsylumError};
use log::{debug, error, info, warn};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client,
};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use serde_json;
use std::{collections::HashSet, io::Write, result, str::FromStr};

pub(crate) const BASE_URL: &str = "https://api.arkhamintelligence.com";

#[derive(Serialize, Deserialize, Debug)]
pub struct ArkhamEntityResp {
    pub entities: Vec<ArkhamEntity>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArkhamEntity {
    pub id: Option<String>,
    pub name: Option<String>,
    pub website: Option<String>,
    pub twitter: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
}

impl PartialEq for ArkhamEntity {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ArkhamEntity {}

impl std::hash::Hash for ArkhamEntity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ArkhamTransaction {
    pub hash: Option<String>,
    pub from_address: Option<String>,
    pub from_entity_id: Option<String>,
    pub from_entity_name: Option<String>,
    pub from_entity_label: Option<String>,
    pub from_entity_type: Option<String>,
    pub from_entity_twitter: Option<String>,
    pub to_address: Option<String>,
    pub to_entity_id: Option<String>,
    pub to_entity_name: Option<String>,
    pub to_entity_label: Option<String>,
    pub to_entity_type: Option<String>,
    pub to_entity_twitter: Option<String>,
    pub token_address: Option<String>,

    pub token_name: Option<String>,
    pub token_symbol: Option<String>,
    pub token_decimals: Option<u8>,
    pub amount: Option<String>,
    pub dollar_amount: Option<String>,

    pub chain: Option<String>,
    pub block_number: Option<i64>,
    pub block_timestamp: Option<String>,
    pub block_hash: Option<String>,
}
impl ArkhamTransaction {
    pub fn new(
        hash: Option<String>,
        from_address: Option<String>,
        from_entity_id: Option<String>,
        from_entity_name: Option<String>,
        from_entity_label: Option<String>,
        from_entity_type: Option<String>,
        from_entity_twitter: Option<String>,
        to_address: Option<String>,
        to_entity_id: Option<String>,
        to_entity_name: Option<String>,
        to_entity_label: Option<String>,
        to_entity_type: Option<String>,
        to_entity_twitter: Option<String>,
        token_address: Option<String>,
        token_name: Option<String>,
        token_symbol: Option<String>,
        token_decimals: Option<u8>,
        amount: Option<String>,
        dollar_amount: Option<String>,
        chain: Option<String>,
        block_number: Option<i64>,
        block_timestamp: Option<String>,
        block_hash: Option<String>,
    ) -> ArkhamTransaction {
        ArkhamTransaction {
            hash,
            from_address,
            from_entity_id,
            from_entity_name,
            from_entity_label,
            from_entity_type,
            from_entity_twitter,
            to_address,
            to_entity_id,
            to_entity_name,
            to_entity_label,
            to_entity_type,
            to_entity_twitter,
            token_address,
            chain,
            block_number,
            block_timestamp,
            block_hash,
            token_name,
            token_symbol,
            token_decimals,
            amount,
            dollar_amount,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ArkhamTokenHolding {
    pub entity_name: Option<String>,
    pub chain: Option<String>,
    pub token_name: Option<String>,
    pub token_symbol: Option<String>,
    pub token_balance: Option<String>,
    pub token_price: Option<String>,
    pub token_balance_usd: Option<String>
}
impl ArkhamTokenHolding {
    pub fn new(
        entity_name: Option<String>,
        chain: Option<String>,
        token_name: Option<String>,
        token_symbol: Option<String>,
        token_balance: Option<String>,
        token_price: Option<String>,
        token_balance_usd: Option<String>,
    ) -> ArkhamTokenHolding {
        ArkhamTokenHolding {
            entity_name,
            chain,
            token_name,
            token_symbol,
            token_balance,
            token_price,
            token_balance_usd,
        }
    }
}

#[derive(Clone)]
pub struct MyArkham {
    client: Client,
}

impl MyArkham {
    pub fn new(secrets_path: Option<String>) -> MyArkham {
        let secret = match secrets_path {
            Some(path) => get_secrets(&path),
            None => get_secrets("Secrets.toml"),
        };
        let key = secret
            .get("ARKHAM_KEY")
            .expect("ARKHAM_KEY must be set in your secret file");
        let header_name = HeaderName::from_str("API-Key").expect("Can't build header name");
        let header_value = HeaderValue::from_str(&key).expect("Can't build header value");
        let mut headers = HeaderMap::new();
        headers.insert(header_name, header_value);
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Can't build client");
        MyArkham { client }
    }
    pub async fn retrieve_enitites(&self, limit: usize) -> Result<Vec<ArkhamEntity>, AsylumError> {
        let mut entities = HashSet::<ArkhamEntity>::new();
        let mut i = 0;
        info!("Retrieving entities...");
        while i < limit {
            let url = format!("{}/tag/top?tag=fund&page={}", BASE_URL, i);
            debug!("{}", url);
            let resp = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| AsylumError::ReqwestError(e))?;
            let body = resp
                .text()
                .await
                .map_err(|e| AsylumError::ReqwestError(e))?;
            // let json: serde_json::Value = serde_json::from_str(&body).expect("Can't parse json");
            let arkham_resp: ArkhamEntityResp = match serde_json::from_str(&body) {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Can't parse json: {}", e);
                    continue;
                }
            };
            // println!("{:?}", arkham_resp);
            for entity in arkham_resp.entities {
                entities.insert(entity);
            }
            i += 1;
        }
        Ok(entities.into_iter().collect())
    }
    pub async fn retrieve_transactions(
        &self,
        entities: &Vec<ArkhamEntity>,
        time_last: &Option<String>,
        limit: &Option<u64>,
    ) -> Result<Vec<ArkhamTransaction>, AsylumError> {
        let mut transactions = Vec::<ArkhamTransaction>::new();
        let joint_entities = entities
            .into_iter()
            .map(|e| e.id.clone().unwrap())
            .collect::<Vec<String>>()
            .join(",");
        let time_last = time_last.clone().unwrap_or("1d".to_owned());
        let limit = limit.unwrap_or(10000);
        let url = format!(
            "{}/transfers?&timeLast={}&limit={}&base={}",
            BASE_URL, time_last, limit, joint_entities
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AsylumError::ReqwestError(e))?;
        let body = resp
            .text()
            .await
            .map_err(|e| AsylumError::ReqwestError(e))?;
        let arkham_resp: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| AsylumError::SerdeError(e))?;
        let transfers = arkham_resp["transfers"].as_array();
        if transfers.is_none() {
            return Err(AsylumError::ArkhamError(
                "Can't find transfers in response".to_owned(),
            ));
        }
        let transfers = transfers.unwrap(); // safe unwrap because if we are here it means that transfers is not None
        for transfer in transfers {
            let hash = transfer
                .get("transactionHash")
                .map(|e| e.as_str().unwrap_or("").to_owned());
            let (
                from_address,
                from_entity_id,
                from_entity_name,
                from_entity_type,
                from_entity_twitter,
                from_entity_label,
            ) = match transfer.get("fromAddress") {
                Some(from) => {
                    let from_address = from
                        .get("address")
                        .map(|e| e.as_str().unwrap_or("").to_owned());
                    let (from_entity_id, from_entity_name, from_entity_type, from_entity_twitter) =
                        match from.get("arkhamEntity") {
                            Some(entity) => {
                                let arkham_entitiy: ArkhamEntity =
                                    serde_json::from_value(entity.clone())
                                        .expect("Can't parse arkham entity");
                                (
                                    arkham_entitiy.id,
                                    arkham_entitiy.name,
                                    arkham_entitiy.type_,
                                    arkham_entitiy.twitter,
                                )
                            }
                            None => (None, None, None, None),
                        };
                    let from_entity_label = match from.get("arkhamLabel") {
                        Some(label) => label
                            .get("name")
                            .map(|e| e.as_str().unwrap_or("").to_owned()),
                        None => None,
                    };

                    (
                        from_address,
                        from_entity_id,
                        from_entity_name,
                        from_entity_type,
                        from_entity_twitter,
                        from_entity_label,
                    )
                }
                None => (None, None, None, None, None, None),
            };
            let (
                to_address,
                to_entity_id,
                to_entity_name,
                to_entity_type,
                to_entity_twitter,
                to_entity_label,
            ) = match transfer.get("toAddress") {
                Some(to) => {
                    let to_address = to
                        .get("address")
                        .map(|e| e.as_str().unwrap_or("").to_owned());
                    let (to_entity_id, to_entity_name, to_entity_type, to_entity_twitter) =
                        match to.get("arkhamEntity") {
                            Some(entity) => {
                                let arkham_entitiy: ArkhamEntity =
                                    serde_json::from_value(entity.clone())
                                        .expect("Can't parse arkham entity");
                                (
                                    arkham_entitiy.id,
                                    arkham_entitiy.name,
                                    arkham_entitiy.type_,
                                    arkham_entitiy.twitter,
                                )
                            }
                            None => (None, None, None, None),
                        };
                    let to_entity_label = match to.get("arkhamLabel") {
                        Some(label) => label
                            .get("name")
                            .map(|e| e.as_str().unwrap_or("").to_owned()),
                        None => None,
                    };

                    (
                        to_address,
                        to_entity_id,
                        to_entity_name,
                        to_entity_type,
                        to_entity_twitter,
                        to_entity_label,
                    )
                }
                None => (None, None, None, None, None, None),
            };
            let token_address = transfer
                .get("tokenAddress")
                .map(|e| e.as_str().unwrap_or("").to_owned());
            let token_name = transfer
                .get("tokenName")
                .map(|e| e.as_str().unwrap_or("").to_owned());
            let token_symbol = transfer
                .get("tokenSymbol")
                .map(|e| e.as_str().unwrap_or("").to_owned());
            let token_decimals = transfer
                .get("tokenDecimals")
                .map(|e| e.as_u64().unwrap_or(0) as u8);
            let amount = transfer
                .get("unitValue")
                .map(|e| e.as_f64().unwrap_or(0.0).to_string());
            let dollar_amount = transfer
                .get("historicalUSD")
                .map(|e| e.as_f64().unwrap_or(0.0).to_string());

            let chain = transfer
                .get("chain")
                .map(|e| e.as_str().unwrap_or("").to_owned());
            let block_number = transfer.get("blockNumber").map(|e| e.as_i64().unwrap_or(0));
            let block_timestamp = transfer
                .get("blockTimestamp")
                .map(|e| e.as_str().unwrap_or("").to_owned());
            let block_hash = transfer
                .get("blockHash")
                .map(|e| e.as_str().unwrap_or("").to_owned());

            let at = ArkhamTransaction::new(
                hash,
                from_address,
                from_entity_id,
                from_entity_name,
                from_entity_label,
                from_entity_type,
                from_entity_twitter,
                to_address,
                to_entity_id,
                to_entity_name,
                to_entity_label,
                to_entity_type,
                to_entity_twitter,
                token_address,
                token_name,
                token_symbol,
                token_decimals,
                amount,
                dollar_amount,
                chain,
                block_number,
                block_timestamp,
                block_hash,
            );
            transactions.push(at);
        }
        Ok(transactions)
    }
    pub async fn retrieve_portfolio(
        &self,
        entity: &ArkhamEntity,
    ) -> Result<Vec<ArkhamTokenHolding>, AsylumError> {
        let entity_id = entity.id.clone().unwrap();
        let entity_name = entity.name.clone().unwrap();
        let timestamp_millis = chrono::Utc::now().timestamp_millis();
        let url = format!("{}/portfolio/entity/{}?time={}", BASE_URL, entity_id, timestamp_millis);
        let resp = self.client.get(&url).send().await.unwrap();
        let body = resp.text().await.unwrap();
        let portfolio: serde_json::Value = serde_json::from_str(&body).unwrap();
        // We get the keys of the portfolio
        let chains = portfolio.as_object().unwrap().keys().collect::<Vec<&String>>();
        let mut holdings = Vec::<ArkhamTokenHolding>::new();
        for chain in chains{
            let chain = chain.to_string();
            let chain_instance = portfolio[&chain].as_object();
            if chain_instance.is_none(){
                continue;
            }
            let chain_instance = chain_instance.unwrap();
            let tokens = chain_instance.keys().collect::<Vec<&String>>();
            for token in tokens{
                let token_info = portfolio[&chain][token].as_object();
                if token_info.is_none(){
                    continue;
                }
                let token_info = token_info.unwrap();
                let token_name = token_info.get("name").map(|e| e.as_str().unwrap_or("").to_owned());
                let token_symbol = token_info.get("symbol").map(|e| e.as_str().unwrap_or("").to_owned());
                let token_balance = token_info.get("balance").map(|e| e.as_f64().unwrap_or(0.0).to_string());
                let token_price = token_info.get("price").map(|e| e.as_f64().unwrap_or(0.0).to_string());
                let token_balance_usd = token_info.get("usd").map(|e| e.as_f64().unwrap_or(0.0).to_string());
                let holding = ArkhamTokenHolding::new(
                    Some(entity_name.to_owned()),
                    Some(chain.to_owned()),
                    token_name,
                    token_symbol,
                    token_balance,
                    token_price,
                    token_balance_usd,
                );
                holdings.push(holding);
            }
        }
        Ok(holdings)
    }
}

#[tokio::test]
async fn test_retrieve_entities() {
    // let ids = retrieve_entities(50).await;
    let arkham = MyArkham::new(None);
    let ids = arkham.retrieve_enitites(50).await;
    // println!("{:?}", ids);
    // we write the vector to a json file
    let mut file = std::fs::File::create("ids.json").expect("Can't create file");
    file.write_all(
        serde_json::to_string(ids.as_ref().unwrap())
            .expect("Can't serialize")
            .as_bytes(),
    )
    .expect("Can't write to file");
    let just_ids = ids
        .unwrap()
        .into_iter()
        .map(|e| e.id.unwrap())
        .collect::<Vec<String>>();
    let mut file = std::fs::File::create("just_ids.txt").expect("Can't create file");
    file.write_all(just_ids.join(",").as_bytes())
        .expect("Can't write to file");
}

#[tokio::test]
async fn test_retrieve_transactions() {
    init_logger(None);
    let arkham = MyArkham::new(None);
    let entities = match arkham.retrieve_enitites(50).await {
        Ok(ids) => ids,
        Err(e) => panic!("Can't retrieve entities: {:?}", e),
    };
    let subset_entities = entities.into_iter().take(10).collect::<Vec<ArkhamEntity>>();
    let transactions = match arkham
        .retrieve_transactions(&subset_entities, &None, &None)
        .await
    {
        Ok(transactions) => transactions,
        Err(e) => panic!("Can't retrieve transactions: {:?}", e),
    };
    info!("{:?}", transactions[0]);
}

#[tokio::test]
pub async fn test_retrieve_portfolio() -> Result<(), AsylumError>{
    init_logger(None);
    let arkham = MyArkham::new(None);
    let entities = match arkham.retrieve_enitites(1).await {
        Ok(ids) => ids,
        Err(e) => panic!("Can't retrieve entities: {:?}", e),
    };
    let entity = entities[0].clone();
    let portfolio = arkham.retrieve_portfolio(&entity).await?;
    info!("{:?}", portfolio);
    Ok(())
}
// async fn
