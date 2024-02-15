use std::io::Write;

use asylum::{arkham::MyArkham, common, Asylum};

#[tokio::main]
async fn main() {
    // let capacity = 100;
    // let config_path = "Config.toml";
    // let secret_path = "Secrets.toml";
    // let log_level = "info";
    // common::init_logger(Some(log_level));
    // Asylum::print_asylum();
    // let asylum = Asylum::new(
    //     Some(config_path),
    //     Some(secret_path),
    //     Some(capacity),
    // )
    // .await;
    // asylum.start().await;

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
