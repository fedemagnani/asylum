use tokio_postgres::{Client, NoTls};
use crate::common::{get_secrets, get_config, init_logger};
use log::{info, warn, error, debug};
use std::{io::Write, fmt::format};

pub struct MyPostgreSQL{
    pub db_name: String,
    client: Client,
    pub table: Option<String>
}

pub struct MyPostgreSQLSetting {
    pub db_name: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub password: String,
}

impl MyPostgreSQLSetting {
    pub fn new(db_name: &str, host: &str, user: &str, port: u16, password: &str) -> MyPostgreSQLSetting {
        MyPostgreSQLSetting {
            db_name: db_name.to_owned(),
            host: host.to_owned(),
            user: user.to_owned(),
            port,
            password: password.to_owned(),
        }
    }

    pub fn new_from_config(secrets_path:Option<&str>, config_path:Option<&str>) -> MyPostgreSQLSetting {
        // Postgres client params here https://docs.rs/postgres/0.19.7/postgres/config/struct.Config.html
        let secret_manager = get_secrets(secrets_path.unwrap_or("Secrets.toml"));
        let config_path = config_path.unwrap_or("Config.toml");
        let config = &get_config(config_path)["db_config"];
        debug!("{:?}", config);
        let host = config["postgres_host"].as_str().expect("postgres_host not found in config file");
        let user = config["postgres_user"].as_str().expect("postgres_user not found in config file");
        let port = match config["postgres_port"].as_str() {
            Some(p) => p.parse::<u16>().expect("postgres_port not parsable from config file"),
            None => {
                warn!("postgres_port not found in config file, using default 5432");
                5432
            },
        };
        let password = secret_manager.get("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD not found in secrets file");
        let db_name = match config["postgres_database"].as_str() {
            Some(d) => d,
            None => {
                warn!("postgres_database not found in config file, using default postgres");
                let db = "postgres";
                db
            }
        };
        MyPostgreSQLSetting {
            db_name: db_name.to_owned(),
            host: host.to_owned(),
            user: user.to_owned(),
            port,
            password: password.to_owned(),
        }
    }
}

impl MyPostgreSQL {
    pub async fn new(secrets_path:Option<&str>, config_path:Option<&str>) -> MyPostgreSQL {
        // Postgres client params here https://docs.rs/postgres/0.19.7/postgres/config/struct.Config.html
        let db_settings = MyPostgreSQLSetting::new_from_config(secrets_path, config_path);
        let (host, user, port, password, db_name) = (db_settings.host, db_settings.user, db_settings.port, db_settings.password, db_settings.db_name);
        warn!("Connecting to postgres and checking existence of database {}", db_name);
        let pg_config = format!("host={} user={} port={} password={} dbname=postgres", host, user, port, password);
        // let client = Client::connect(&pg_config, NoTls).expect("Failed to connect to postgres");
        let (client, connection) = tokio_postgres::connect(&pg_config, NoTls).await.expect("Failed to connect to postgres");
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Connection error: {}", e);
            }
        });
        info!("Connecting to postgres and checking existence of database {}", db_name);
        // We create the dataset if it is not already created
        let mut pg = MyPostgreSQL{db_name:db_name.to_owned(), client, table:None};
        pg.create_db(&db_name).await.expect("Failed to create database");
        let pg_config = format!("host={} user={} port={} password={} dbname={}", host, user, port, password, db_name);
        // let client = Client::connect(&pg_config, NoTls).expect("Failed to connect to postgres");
        let (client, connection) = tokio_postgres::connect(&pg_config, NoTls).await.expect("Failed to connect to postgres");
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Connection error: {}", e);
            }
        });
        
        info!("Connected to postgres and database {}", db_name);
        pg.client = client;
        pg
    }

    pub async fn create_db(&mut self, db_name: &str)-> Result<(), postgres::Error> {
        // Use format! to safely create the SQL command
        let check_db_command = format!("SELECT 1 FROM pg_database WHERE datname = '{}'", db_name);
        // Check if the database exists
        let db_exists = self.client.query_one(check_db_command.as_str(), &[]).await.is_ok();
        // If the database doesn't exist, create it
        if !db_exists {
            let create_db_command = format!("CREATE DATABASE {}", db_name);
            self.client.execute(create_db_command.as_str(), &[]).await?;
        }else {
            warn!("Database {} already exists", db_name);
        }
        Ok(())
    }

    pub async fn list_dbs(&mut self) -> Result<(), postgres::Error> {
        let list_dbs_command = "SELECT datname FROM pg_database WHERE datistemplate = false";
        let rows = self.client.query(list_dbs_command, &[]).await?;
        for row in rows {
            let db_name: &str = row.get(0);
            debug!("{}", db_name);
        }
        Ok(())
    }

    pub async fn remove_db(&mut self, db_name: &str) -> Result<(), postgres::Error> {
        let remove_db_command = format!("DROP DATABASE {}", db_name);
        self.client.execute(remove_db_command.as_str(), &[]).await?;
        Ok(())
    }

    pub async fn create_table(&mut self, table_name: &str, columns: &str) -> Result<(), postgres::Error> {
        // We check if the table exists, if not we create it
        let create_table_command = format!("CREATE TABLE IF NOT EXISTS {} ({})", table_name, columns);
        self.client.execute(create_table_command.as_str(), &[]).await?;
        self.table = Some(table_name.to_owned());
        Ok(())
    }

    async fn list_tables(&mut self) -> Result<(), postgres::Error> {
        let list_tables_command = "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'";
        let rows = self.client.query(list_tables_command, &[]).await?;
        for row in rows {
            let table_name: &str = row.get(0);
            debug!("{}", table_name);
        }
        Ok(())
    }

    async fn remove_table(&mut self, table_name: &str) -> Result<(), postgres::Error> {
        let remove_table_command = format!("DROP TABLE {}", table_name);
        self.client.execute(remove_table_command.as_str(), &[]).await?;
        self.table = None;
        Ok(())
    }

    async fn table(&mut self, table_name: &str) -> Option<String> {
        let table_exists_command = format!("SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = '{}'", table_name);
        let table_exists = self.client.query_one(table_exists_command.as_str(), &[]).await.is_ok();
        if table_exists {
            self.table = Some(table_name.to_owned());
            Some(table_name.to_owned())
        }else {
            warn!("Table {} does not exist", table_name);
            None
        }
    }

    pub async fn execute(&mut self, command: &str, params: &[&(dyn postgres::types::ToSql + Sync)]) -> Result<u64, postgres::Error> {
        self.client.execute(command, params).await
    }

    pub async fn query(&self, statement:&str) -> Result<Vec<postgres::Row>, postgres::Error> {
        self.client.query(statement, &[]).await
    }
}

#[tokio::test]
async fn test_postgres() {
    init_logger(None);
    use crate::postgres::MyPostgreSQL;
    let mut pg = MyPostgreSQL::new(None, None).await;
    let res = pg.create_db("test").await;
    match res {
        Ok(_) => info!("Database created"),
        Err(e) => error!("Error creating database: {}", e),
    }
    let res = pg.list_dbs().await;
    match res {
        Ok(_) => info!("Database listed"),
        Err(e) => error!("Error listing databases: {}", e),
    }
    let res = pg.remove_db("test").await;
    match res {
        Ok(_) => info!("Database removed"),
        Err(e) => error!("Error removing database: {}", e),
    }
    let res = pg.list_dbs().await;
    match res {
        Ok(_) => info!("Database listed"),
        Err(e) => error!("Error listing databases: {}", e),
    }
}