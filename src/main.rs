use async_ctrlc::CtrlC;
use async_std::prelude::*;
// use async_std::task::JoinHandle;
use dotenv::dotenv;
use futures_channel::mpsc;
use log::info;
use serde::Deserialize;
use smol_str::SmolStr;
use sqlx::{pool::PoolOptions, Postgres};
use std::time::Duration;

mod changefeed;
mod db_cursor;
mod options;
mod publisher;

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[derive(Deserialize, Debug)]
struct Config {
  pub database_url: String,
  pub tables: String,
  pub amqp_url: String,
  pub amqp_exchange: String,
}

pub struct PublishPayload {
  pub table_name: SmolStr,
  pub payload: Vec<u8>,
}

pub struct CursorPayload {
  pub table_name: SmolStr,
  pub cursor: SmolStr,
}

pub enum BridgeEvent {
  Publish(PublishPayload),
  Cursor(CursorPayload),
  Stop,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  dotenv().ok();

  env_logger::init();

  let config: Config = envy::from_env()?;

  info!("config: {:?}", config);

  let pool = PoolOptions::<Postgres>::new()
    .max_connections(4)
    .min_connections(2)
    .max_lifetime(None)
    .idle_timeout(None)
    .connect_timeout(Duration::from_secs(5))
    .test_before_acquire(false)
    .connect(config.database_url.as_str())
    .await?;

  db_cursor::create_cursor_table(&pool).await?;

  let (sender, receiver) = mpsc::unbounded();

  let cloned_pool = pool.clone();
  let amqp_url = config.amqp_url.clone();
  let amqp_exchange = config.amqp_exchange.clone();

  let publisher = async_std::task::spawn(async move {
    publisher::start(cloned_pool, receiver, &amqp_url, &amqp_exchange).await;
  });

  let pool = pool.clone();
  let sender = sender.clone();
  let table = config.tables.into();

  let cdc = async_std::task::spawn(async move {
    changefeed::start(pool, table, sender).await;
  });

  let ctrlc = CtrlC::new().expect("cannot create Ctrl+C handler?");

  ctrlc.race(async move { cdc.race(publisher).await }).await;

  info!("stop fetching changefeeds...");

  info!("done");

  Ok(())
}
