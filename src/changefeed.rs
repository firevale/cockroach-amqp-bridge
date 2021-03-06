use crate::db_cursor;
use crate::options::{ChangefeedOptions, Envelope};
use crate::{BridgeEvent, CursorPayload, PublishPayload};

use futures::StreamExt;
use futures_channel::mpsc;
use log::{error, info};
use serde::Deserialize;
use serde_json::json;
use smol_str::SmolStr;
use sqlx::{pool::Pool, FromRow, Postgres};

pub struct Changefeed {
  pub pool: Pool<Postgres>,
  pub feed_options: ChangefeedOptions,
  pub sender: mpsc::UnboundedSender<BridgeEvent>,
  pub cursor_saved: bool,
  pub cursor: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ChangeData {
  pub table: Option<String>,
  pub key: Option<Vec<u8>>,
  pub value: Option<Vec<u8>>,
}

#[derive(Debug, Deserialize)]
struct Resolved {
  resolved: SmolStr,
}

impl Changefeed {
  async fn infinite_fetch(&mut self) -> anyhow::Result<()> {
    let query = self.feed_options.query_string();

    let mut conn = self.pool.acquire().await?;

    let mut s = sqlx::query(&query).fetch_many(&mut conn);

    while let Some(result) = s.next().await {
      match result {
        Ok(either::Either::Right(row)) => {
          match ChangeData::from_row(&row).unwrap() {
            ChangeData {
              table: Some(table),
              key: Some(key),
              value: Some(value),
            } => {
              let key = String::from_utf8(key).expect("invalid key encoding");

              let value: serde_json::Value = serde_json::from_slice(value.as_slice())
                .expect("invalid value encoding");

              if let Some(serde_json::Value::String(cursor)) = value.get("updated") {
                self.cursor = cursor.clone();
              }

              let payload = serde_json::to_vec(&json!({
                "key": key,
                "value": value
              }))?;

              let payload = PublishPayload {
                table_name: table.into(),
                payload,
              };

              let event = BridgeEvent::Publish(payload);

              self.sender.unbounded_send(event)?;

              self.cursor_saved = false;
            }

            ChangeData {
              table: None,
              key: None,
              value: Some(value),
            } => {
              let res: Resolved = serde_json::from_slice(value.as_slice())?;

              if !self.cursor_saved {
                let payload = CursorPayload {
                  table_name: self.feed_options.table_name.clone(),
                  cursor: self.cursor.clone().into(),
                };

                let event = BridgeEvent::Cursor(payload);

                self.sender.unbounded_send(event)?;

                self.cursor_saved = true;
              }

              self.feed_options.cursor = Some(res.resolved);
            }

            _ => break,
          };
        }

        Ok(either::Either::Left(_)) => {
          info!("Done fetch change feed.");
          break;
        }

        Err(sqlx::Error::Io(err))
          if err.kind() == std::io::ErrorKind::ConnectionAborted =>
        {
          error!(
            "Database connection [{}] lost",
            self.feed_options.table_name.as_str()
          );
          break;
        }

        Err(e) => {
          error!("something went wrong: {:?}", e);
          break;
        }
      }
    }

    error!(
      "stop listening cdc on table: {}",
      self.feed_options.table_name
    );

    self.sender.unbounded_send(BridgeEvent::Stop)?;

    Ok(())
  }
}

pub async fn start(
  pool: Pool<Postgres>,
  table_name: SmolStr,
  sender: mpsc::UnboundedSender<BridgeEvent>,
) {
  info!("fetching changefeed of table: {}", table_name);

  let cursor = db_cursor::get_cursor(&pool, &table_name)
    .await
    .expect("can not fetch cursor from database");

  let feed_options = ChangefeedOptions {
    table_name,
    cursor,
    envelope: Envelope::default(),
  };

  let mut feed = Changefeed {
    pool,
    sender,
    feed_options,
    cursor_saved: true,
    cursor: "".to_string(),
  };

  feed.infinite_fetch().await.expect("feed stopped");
}
