use async_ctrlc::CtrlC;
use async_std::task::JoinHandle;
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
  env_logger::init();

  let config: Config = envy::from_env()?;

  let tables: Vec<&str> = config.tables.split(',').collect();

  let pool = PoolOptions::<Postgres>::new()
    .max_connections(tables.len() as u32 * 2_u32)
    .min_connections(tables.len() as u32)
    .max_lifetime(None)
    .idle_timeout(None)
    .connect_timeout(Duration::from_secs(10))
    .test_before_acquire(false)
    .connect(config.database_url.as_str())
    .await?;

  db_cursor::create_cursor_table(&pool).await?;

  let (sender, receiver) = mpsc::unbounded();

  let cloned_pool = pool.clone();
  let amqp_uri = config.amqp_url.clone();
  let amqp_exchange = config.amqp_exchange.clone();

  let publisher = async_std::task::spawn(async move {
    publisher::start(cloned_pool, receiver, &amqp_uri, &amqp_exchange).await;
  });

  let handles: Vec<JoinHandle<()>> = tables
    .iter()
    .cloned()
    .map(|table| {
      let pool = pool.clone();
      let sender = sender.clone();
      let table = table.into();
      async_std::task::spawn(async move {
        changefeed::start(pool, table, sender).await;
      })
    })
    .collect();

  CtrlC::new().expect("cannot create Ctrl+C handler?").await;

  info!("stop fetching changefeeds...");
  drop(handles);

  info!("notify publisher to stop");
  sender.unbounded_send(BridgeEvent::Stop)?;
  publisher.await;

  info!("done");

  Ok(())
}
