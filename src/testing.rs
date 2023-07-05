use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

use crate::db;

pub async fn database() -> SqlitePool {
    let db = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::init(&db).await.unwrap();
    db
}

pub async fn insert_visitor(db: &SqlitePool, nick: &str, group: Option<&str>) {
    sqlx::query(r#"INSERT INTO visitor (created_at, ip, nick, "group") VALUES (CURRENT_TIMESTAMP, '127.0.0.1:8080', $1, $2)"#)
        .bind(nick)
        .bind(group)
        .execute(db)
        .await
        .unwrap();
}
