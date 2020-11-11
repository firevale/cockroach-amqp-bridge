use crate::db_cursor;
use crate::{BridgeEvent, PublishPayload};
use async_std::stream::StreamExt;
use futures_channel::mpsc;
use lapin::{options::*, BasicProperties, Channel, Connection, ConnectionProperties};
use log::info;
use sqlx::{pool::Pool, Postgres};

pub struct Publisher {
  pub amqp_uri: String,
  pub exchange: String,
  pub channel: Option<Channel>,
}

impl Publisher {
  async fn create_channel(&mut self) -> anyhow::Result<()> {
    let conn = Connection::connect(
      self.amqp_uri.as_str(),
      ConnectionProperties::default().with_default_executor(8),
    )
    .await?;

    let channel = conn.create_channel().await?;

    self.channel = Some(channel);

    Ok(())
  }

  async fn create_channel_if_needed(&mut self) -> anyhow::Result<()> {
    match self.channel {
      Some(ref channel) => {
        if !channel.status().connected() {
          // reset channel if disconnected
          self.channel = None;
          self.create_channel().await?;
        }
      }
      None => {
        self.create_channel().await?;
      }
    }

    Ok(())
  }

  pub async fn publish(&mut self, payload: PublishPayload) {
    self
      .create_channel_if_needed()
      .await
      .expect("connect amqp failed");

    if let Some(ref channel) = self.channel {
      channel
        .basic_publish(
          &self.exchange,
          &payload.table_name,
          BasicPublishOptions::default(),
          payload.payload,
          BasicProperties::default(),
        )
        .await
        .unwrap()
        .await
        .unwrap();
    } else {
      panic!("unexpected error: channel is none");
    }
  }
}

pub async fn start(
  pool: Pool<Postgres>,
  mut receiver: mpsc::UnboundedReceiver<BridgeEvent>,
  amqp_uri: &str,
  amqp_exchange: &str,
) {
  let mut publisher = Publisher {
    amqp_uri: amqp_uri.to_owned(),
    exchange: amqp_exchange.to_owned(),
    channel: None,
  };

  while let Some(event) = receiver.next().await {
    match event {
      BridgeEvent::Cursor(payload) => {
        db_cursor::save_cursor(&pool, &payload.table_name, &payload.cursor)
          .await
          .expect("save cursor failed");
      }

      BridgeEvent::Publish(payload) => {
        info!(
          "forward change data to amqp, route_key={}",
          payload.table_name
        );

        publisher.publish(payload).await;
      }

      BridgeEvent::Stop => {
        info!("stop publishing to amqp.");
        break;
      }
    }
  }
}
