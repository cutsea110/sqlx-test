use futures::TryStreamExt; // try_next()
use sqlx::postgres::PgPoolOptions;
use sqlx::prelude::*;

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

    Ok(())
}
