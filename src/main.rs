use sqlx::postgres::PgConnection;
use sqlx::Connection;

#[derive(Debug)]
enum DomainError {
    ConnectFailed,
    SqlxError(sqlx::Error),
}

type Result<T> = std::result::Result<T, DomainError>;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
struct User {
    id: i32,
    name: String,
    email: String,
}

struct UserRepo {
    conn: PgConnection,
}

impl UserRepo {
    pub async fn new(conn: PgConnection) -> Result<Self> {
        Ok(Self { conn })
    }

    async fn add_user(&mut self, name: String, email: String) -> Result<User> {
        self.conn
            .transaction(|txn| {
                Box::pin(async move {
                    sqlx::query_as::<_, User>(
                        "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *",
                    )
                    .bind(name)
                    .bind(email)
                    .fetch_one(&mut **txn)
                    .await
                })
            })
            .await
            .map_err(DomainError::SqlxError)
    }

    async fn collect_users(&mut self) -> Result<Vec<User>> {
        self.conn
            .transaction(|txn| {
                Box::pin(async move {
                    sqlx::query_as("SELECT * FROM users")
                        .fetch_all(&mut **txn)
                        .await
                })
            })
            .await
            .map_err(DomainError::SqlxError)
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    let db_url =
        std::env::var("DATABASE_URL").expect("Env var DATABASE_URL is required for this test");
    let conn = PgConnection::connect(&db_url)
        .await
        .map_err(|_| DomainError::ConnectFailed)?;
    let mut user_db = UserRepo::new(conn).await?;

    let id = user_db
        .add_user("John".to_string(), "john@google.com".to_string())
        .await?;

    println!("id: {:?}", id);

    let users = user_db.collect_users().await?;

    println!("{:#?}", users);

    Ok(())
}
