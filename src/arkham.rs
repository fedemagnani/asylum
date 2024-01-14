use serde::{Deserialize, Serialize, ser::SerializeStruct};
use reqwest::{header::{HeaderName, HeaderValue, HeaderMap}, Client};
use serde_json;
use std::{collections::HashSet, str::FromStr, io::Write, result};
use crate::common::{get_secrets, AsylumError, init_logger};
use log::{info, warn, error, debug};


pub(crate) const BASE_URL : &str = "https://api.arkhamintelligence.com";

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
    pub type_ : Option<String>,
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

#[derive(Debug, Clone)]
pub struct ArkhamTransaction {
    pub hash: Option<String>,
    pub from_address: Option<String>,
    pub from_entity_id: Option<String>,
    pub from_entity_name: Option<String>,
    pub from_entity_label: Option<String>,
    pub from_entity_type : Option<String>,
    pub from_entity_twitter: Option<String>,
    pub to_address: Option<String>,
    pub to_entity_id: Option<String>,
    pub to_entity_name: Option<String>,
    pub to_entity_label: Option<String>,
    pub to_entity_type : Option<String>,
    pub to_entity_twitter: Option<String>,
    pub token_address: Option<String>,
    pub chain: Option<String>,
    pub block_number: Option<i64>,
    pub block_timestamp: Option<String>,
    pub block_hash: Option<String>,
}
impl ArkhamTransaction {
    pub fn new(hash: Option<String>,
    from_address: Option<String>,
    from_entity_id: Option<String>,
    from_entity_name: Option<String>,
    from_entity_label: Option<String>,
    from_entity_type : Option<String>,
    from_entity_twitter: Option<String>,
    to_address: Option<String>,
    to_entity_id: Option<String>,
    to_entity_name: Option<String>,
    to_entity_label: Option<String>,
    to_entity_type : Option<String>,
    to_entity_twitter: Option<String>,
    token_address: Option<String>,
    chain: Option<String>,
    block_number: Option<i64>,
    block_timestamp: Option<String>,
    block_hash: Option<String>) -> ArkhamTransaction {
        ArkhamTransaction{
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
        }
    }
}

#[derive(Clone)]
pub struct MyArkham{
    client: Client
}

impl MyArkham {
    pub fn new(secrets_path:Option<String>) -> MyArkham {
        let secret = match secrets_path {
            Some(path) => get_secrets(&path),
            None => get_secrets("Secrets.toml")
        };
        let key = secret.get("ARKHAM_KEY").expect("ARKHAM_KEY must be set in your secret file");
        let header_name = HeaderName::from_str("API-Key").expect("Can't build header name");
        let header_value = HeaderValue::from_str(&key).expect("Can't build header value");
        let mut headers = HeaderMap::new();
        headers.insert(header_name, header_value);
        let client = Client::builder()
            .default_headers(headers)
            .build().expect("Can't build client");
        MyArkham{client}
    }
    pub async fn retrieve_enitites(&self, limit:usize) -> Result<Vec<ArkhamEntity>, AsylumError>{
        let mut entities = HashSet::<ArkhamEntity>::new();
        let mut i = 0;
        while i < limit {
            info!("Fetching page {} of {}", i, limit);
            let url = format!("{}/tag/top?tag=fund&page={}", BASE_URL, i);
            debug!("{}", url);
            let resp = self.client.get(&url).send().await.map_err(|e| AsylumError::ReqwestError(e))?;
            let body = resp.text().await.map_err(|e| AsylumError::ReqwestError(e))?;
            // let json: serde_json::Value = serde_json::from_str(&body).expect("Can't parse json");
            let arkham_resp: ArkhamEntityResp = match serde_json::from_str(&body) {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Can't parse json: {}", e);
                    continue;
                }
            };
            // println!("{:?}", arkham_resp);
            for entity in arkham_resp.entities{
                entities.insert(entity);
            }
            i += 1;
        }
        Ok(entities.into_iter().collect())
    }
    pub async fn retrieve_transactions(&self, entities:&Vec<ArkhamEntity>,time_last:&Option<String>,limit:&Option<u64>) -> Result<Vec<ArkhamTransaction>, AsylumError>{
        let mut transactions = Vec::<ArkhamTransaction>::new();
        let joint_entities = entities.into_iter().map(|e| e.id.clone().unwrap()).collect::<Vec<String>>().join(",");
        let time_last = time_last.clone().unwrap_or("1d".to_owned());
        let limit = limit.unwrap_or(10000);
        let url = format!("{}/transfers?&timeLast={}&limit={}&base={}", BASE_URL, time_last, limit, joint_entities);
        let resp = self.client.get(&url).send().await.map_err(|e| AsylumError::ReqwestError(e))?;
        let body = resp.text().await.map_err(|e| AsylumError::ReqwestError(e))?;
        let arkham_resp: serde_json::Value = serde_json::from_str(&body).map_err(|e| AsylumError::SerdeError(e))?;
        let transfers = arkham_resp["transfers"].as_array().expect("Can't get transfers");
        for transfer in transfers {
            let hash = transfer.get("transactionHash").map(|e| e.as_str().unwrap_or("").to_owned());
            let (from_address, from_entity_id, from_entity_name, from_entity_type, from_entity_twitter, from_entity_label) = match transfer.get("fromAddress") {
                Some(from) => {
                    let from_address = from.get("address").map(|e| e.as_str().unwrap_or("").to_owned());
                    let (from_entity_id, from_entity_name, from_entity_type, from_entity_twitter) = match from.get("arkhamEntity") {
                        Some(entity) => {
                            let arkham_entitiy: ArkhamEntity = serde_json::from_value(entity.clone()).expect("Can't parse arkham entity");
                            (arkham_entitiy.id, arkham_entitiy.name, arkham_entitiy.type_, arkham_entitiy.twitter)
                        }
                        None => (None, None, None, None),
                    };
                    let from_entity_label = match from.get("arkhamLabel") {
                        Some(label) => label.get("name").map(|e| e.as_str().unwrap_or("").to_owned()),
                        None => None,
                    };
                    
                    (from_address, from_entity_id, from_entity_name, from_entity_type, from_entity_twitter, from_entity_label)
                }
                None => (None, None, None, None, None, None),
            };
            let (to_address, to_entity_id, to_entity_name, to_entity_type, to_entity_twitter, to_entity_label) = match transfer.get("toAddress") {
                Some(to) => {
                    let to_address = to.get("address").map(|e| e.as_str().unwrap_or("").to_owned());
                    let (to_entity_id, to_entity_name, to_entity_type, to_entity_twitter) = match to.get("arkhamEntity") {
                        Some(entity) => {
                            let arkham_entitiy: ArkhamEntity = serde_json::from_value(entity.clone()).expect("Can't parse arkham entity");
                            (arkham_entitiy.id, arkham_entitiy.name, arkham_entitiy.type_, arkham_entitiy.twitter)
                        }
                        None => (None, None, None, None),
                    };
                    let to_entity_label = match to.get("arkhamLabel") {
                        Some(label) => label.get("name").map(|e| e.as_str().unwrap_or("").to_owned()),
                        None => None,
                    };
                    
                    (to_address, to_entity_id, to_entity_name, to_entity_type, to_entity_twitter, to_entity_label)
                }
                None => (None, None, None, None, None, None),
            };
            let token_address = transfer.get("tokenAddress").map(|e| e.as_str().unwrap_or("").to_owned());
            let chain = transfer.get("chain").map(|e| e.as_str().unwrap_or("").to_owned());
            let block_number = transfer.get("blockNumber").map(|e| e.as_i64().unwrap_or(0));
            let block_timestamp = transfer.get("blockTimestamp").map(|e| e.as_str().unwrap_or("").to_owned());
            let block_hash = transfer.get("blockHash").map(|e| e.as_str().unwrap_or("").to_owned());
            let at = ArkhamTransaction::new(hash, from_address, from_entity_id, from_entity_name, from_entity_label, from_entity_type, from_entity_twitter, to_address, to_entity_id, to_entity_name, to_entity_label, to_entity_type, to_entity_twitter, token_address, chain, block_number, block_timestamp, block_hash);
            transactions.push(at);
        }
        Ok(transactions)
    } 
}

#[tokio::test]
async fn test_retrieve_entities(){
    // let ids = retrieve_entities(50).await;
    let arkham = MyArkham::new(None);
    let ids = arkham.retrieve_enitites(50).await;
    // println!("{:?}", ids);
    // we write the vector to a json file
    let mut file = std::fs::File::create("ids.json").expect("Can't create file");
    file.write_all(serde_json::to_string(ids.as_ref().unwrap()).expect("Can't serialize").as_bytes()).expect("Can't write to file");
    let just_ids = ids.unwrap().into_iter().map(|e| e.id.unwrap()).collect::<Vec<String>>();
    let mut file = std::fs::File::create("just_ids.txt").expect("Can't create file");
    file.write_all(just_ids.join(",").as_bytes()).expect("Can't write to file");
}

#[tokio::test]
async fn test_retrieve_transactions(){
    init_logger(None);
    let arkham = MyArkham::new(None);
    let entities = match arkham.retrieve_enitites(50).await {
        Ok(ids) => ids,
        Err(e) => panic!("Can't retrieve entities: {:?}", e),
    };
    let subset_entities = entities.into_iter().take(10).collect::<Vec<ArkhamEntity>>();
    let transactions = match arkham.retrieve_transactions(&subset_entities, &None, &None).await {
        Ok(transactions) => transactions,
        Err(e) => panic!("Can't retrieve transactions: {:?}", e),
    };
    info!("{:?}", transactions[0]);

}

// async fn 

