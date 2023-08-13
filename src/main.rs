use sqlx::postgres::PgConnection;
use sqlx::Connection;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
struct User {
    id: i32,
    name: String,
    email: String,
}

async fn collect_users(conn: &mut PgConnection) -> sqlx::Result<Vec<User>> {
    conn.transaction(|txn| {
        Box::pin(async move {
            sqlx::query_as("SELECT * FROM users")
                .fetch_all(&mut **txn)
                .await
        })
    })
    .await
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url =
        std::env::var("DATABASE_URL").expect("Env var DATABASE_URL is required. for this test");
    let mut conn = PgConnection::connect(&db_url).await?;
    let users = collect_users(&mut conn).await?;
    println!("{:#?}", users);

    Ok(())
}
