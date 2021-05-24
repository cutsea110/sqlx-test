use sqlx::postgres::PgPoolOptions;

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
    assert_eq!(row.0, 150);

    let mut tx = conn.begin().await?;
    let c = sqlx::query("DELETE FROM commenttree")
        .execute(&mut tx)
        .await?;
    println!("{:#?}", c.rows_affected());
    tx.rollback().await?;

    Ok(())
}
