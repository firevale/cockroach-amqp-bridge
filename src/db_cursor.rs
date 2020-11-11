use anyhow::anyhow;
use smol_str::SmolStr;
use sqlx::{pool::Pool, Postgres};

pub async fn create_cursor_table(pool: &Pool<Postgres>) -> anyhow::Result<()> {
  let query = r#" 
    CREATE TABLE IF NOT EXISTS _amqp_bridge_cursors (
      "table_name" VARCHAR(100) PRIMARY KEY,
      "cursor" VARCHAR(30)
    );
  "#;

  sqlx::query(query).execute(pool).await?;

  Ok(())
}

pub async fn get_cursor(
  pool: &Pool<Postgres>,
  table: &str,
) -> anyhow::Result<Option<SmolStr>> {
  let query = r#" 
    SELECT cursor from _amqp_bridge_cursors where table_name = $1; 
  "#;

  let res: Result<(String,), sqlx::Error> =
    sqlx::query_as(query).bind(table).fetch_one(pool).await;

  match res {
    Ok(row) => Ok(Some(row.0.into())),
    Err(sqlx::Error::RowNotFound) => Ok(None),
    Err(e) => Err(anyhow!(e)),
  }
}

pub async fn save_cursor(
  pool: &Pool<Postgres>,
  table: &str,
  cursor: &str,
) -> anyhow::Result<()> {
  let query = r#" 
    UPSERT INTO _amqp_bridge_cursors (table_name, cursor) VALUES ($1, $2);
  "#;

  sqlx::query(query)
    .bind(table)
    .bind(cursor)
    .execute(pool)
    .await?;

  Ok(())
}
