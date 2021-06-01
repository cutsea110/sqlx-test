use anyhow::Result;
use futures::TryStreamExt; // try_next()
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
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

async fn new_conn(conn_str: &str) -> Result<PgPool> {
    let conn = PgPoolOptions::new()
        .max_connections(5)
        .connect(conn_str)
        .await?;

    Ok(conn)
}

async fn insert_account(conn: &sqlx::PgPool) -> Result<i32> {
    let mut tx = conn.begin().await?;

    let row: (i32,) = sqlx::query_as(
        r#"
INSERT INTO accounts
  ( account_name, first_name, last_name, email, password_hash, portrait_image, hourly_rate)
   VALUES
  ( $1, $2, $3, $4, $5, $6, $7)
  RETURNING account_id
"#,
    )
    .bind(Some(String::from("Katsutoshi Itoh")))
    .bind(Some(String::from("Itoh")))
    .bind(Some(String::from("Katsutoshi")))
    .bind(Some(String::from("cutsea110@gmail.com")))
    .bind(None as Option<String>)
    .bind(None as Option<Vec<u8>>)
    .bind(None as Option<f32>)
    .fetch_one(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(row.0)
}

async fn select_const(conn: &sqlx::PgPool) -> Result<i64> {
    let tx = conn.begin().await?;

    let row: (i64,) = sqlx::query_as("SELECT $1")
        .bind(150_i64)
        .fetch_one(conn)
        .await?;

    tx.rollback().await?;

    Ok(row.0)
}

async fn delete_all_commenttree(conn: &sqlx::PgPool) -> Result<u64> {
    let mut tx = conn.begin().await?;

    let c = sqlx::query("DELETE FROM commenttree")
        .execute(&mut tx)
        .await?;

    tx.commit().await?;

    Ok(c.rows_affected())
}

async fn select_all_accounts_name(conn: &sqlx::PgPool) -> Result<Vec<Option<String>>> {
    let rows = select_all_acounts_1(conn).await?;
    let mut v = vec![];

    for x in rows {
        v.push(x.account_name);
    }

    Ok(v)
}

async fn select_all_acounts_1(conn: &sqlx::PgPool) -> Result<Vec<Accounts>> {
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
        .fetch(conn);
    let mut v = vec![];

    while let Some(row) = rows.try_next().await? {
        v.push(row)
    }

    Ok(v)
}

async fn select_all_accounts_2(conn: &sqlx::PgPool) -> Result<Vec<Accounts>> {
    let mut rows = sqlx::query_as::<_, Accounts>(r#"SELECT * FROM accounts"#).fetch(conn);
    let mut v = vec![];

    while let Some(row) = rows.try_next().await? {
        v.push(row);
    }

    Ok(v)
}

#[async_std::main]
async fn main() -> Result<()> {
    let conn = new_conn("postgres://admin:admin@localhost:15432/sampledb").await?;

    let n = select_const(&conn).await?;

    println!("SELECT: {}", n);

    let c = delete_all_commenttree(&conn).await?;

    println!("DELETE: {:?}", c);

    let row = insert_account(&conn).await?;
    println!("INSERTED: {}", row);

    let rows = select_all_accounts_name(&conn).await?;
    for name in rows {
        println!("{:?}", name);
    }

    let rows = select_all_acounts_1(&conn).await?;
    for row in rows {
        println!("{:#?}", row);
    }

    let rows = select_all_accounts_2(&conn).await?;
    for row in rows {
        println!("{:#?}", row);
    }

    let select_task = async_std::task::spawn(async move {
        let result = select_all_accounts_2(&conn).await;
        match result {
            Ok(s) => {
                println!("ASYNC!");
                println!("{:#?}", s);
            }
            Err(e) => println!("Error select: {:?}", e),
        }
    });
    async_std::task::block_on(select_task);

    Ok(())
}
