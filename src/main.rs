use sqlx::postgres::PgConnection;
use sqlx::Connection;

#[derive(Debug)]
enum DomainError {
    SqlxError(sqlx::Error),
}

type Result<T> = std::result::Result<T, DomainError>;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
struct User {
    id: i32,
    name: String,
    email: String,
}

struct Usecase {
    conn: PgConnection,
}

impl Usecase {
    pub async fn new(conn_str: &str) -> Result<Self> {
        let conn = PgConnection::connect(conn_str)
            .await
            .map_err(DomainError::SqlxError)?;

        Ok(Self { conn })
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
        std::env::var("DATABASE_URL").expect("Env var DATABASE_URL is required. for this test");

    let mut usecase = Usecase::new(&db_url).await?;

    let users = usecase.collect_users().await?;

    println!("{:#?}", users);

    Ok(())
}
