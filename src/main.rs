use futures::TryStreamExt; // try_next()
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::prelude::*;

//
// https://docs.rs/sqlx/0.4.0-beta.1/sqlx/postgres/types/index.html
//
#[derive(Debug, sqlx::FromRow)]
struct Accounts {
    account_id: i32,
    account_name: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
    password_hash: Option<String>,
    portrait_image: Option<Vec<u8>>,
    hourly_rate: Option<f32>,
}

#[async_std::main]
async fn main() -> Result<(), sqlx::Error> {
    let conn = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://admin:admin@localhost:15432/sampledb")
        .await?;

    let row: (i64,) = sqlx::query_as("SELECT $1")
        .bind(150_i64)
        .fetch_one(&conn)
        .await?;

    println!("SELECT: {}", row.0);

    let mut tx = conn.begin().await?;
    let c = sqlx::query("DELETE FROM commenttree")
        .execute(&mut tx)
        .await?;
    println!("DELETE: {:?}", c.rows_affected());

    tx.rollback().await?;

    let mut rows = sqlx::query("SELECT account_name FROM accounts").fetch(&conn);
    while let Some(row) = rows.try_next().await? {
        let name: &str = row.try_get("account_name")?;
        println!("{:?}", name);
    }

    let mut rows = sqlx::query(r#"SELECT * FROM accounts"#)
        .map(|row: PgRow| Accounts {
            account_id: row.get(0),
            account_name: row.get(1),
            first_name: row.get(2),
            last_name: row.get(3),
            email: row.get(4),
            password_hash: row.get(5),
            portrait_image: row.get(6),
            hourly_rate: row.get(7),
        })
        .fetch(&conn);
    while let Some(row) = rows.try_next().await? {
        println!("{:#?}", row);
    }

    let mut rows = sqlx::query_as::<_, Accounts>(r#"SELECT * FROM accounts"#).fetch(&conn);
    while let Some(row) = rows.try_next().await? {
        println!("{:#?}", row);
    }

    Ok(())
}
