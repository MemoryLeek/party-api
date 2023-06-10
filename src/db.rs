use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

#[derive(sqlx::FromRow)]
pub struct Visitor {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub ip: String,

    pub nick: String,
    pub group: Option<String>,
    pub email: Option<String>,
    pub extra: Option<String>,
}

pub async fn init(db: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS visitor (
  id INTEGER PRIMARY KEY,
  created_at TEXT NOT NULL,
  ip TEXT NOT NULL,

  nick TEXT NOT NULL UNIQUE,
  "group" TEXT,
  email TEXT,
  extra TEXT
) STRICT;"#,
    )
    .execute(db)
    .await?;

    Ok(())
}
