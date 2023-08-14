use core::{future::Future, marker::Send, pin::Pin};

use sqlx::postgres::PgConnection;
use sqlx::{Connection, Postgres, Transaction};

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

struct Usecase {
    conn: PgConnection,
}

impl Usecase {
    pub async fn new(conn_str: &str) -> Result<Self> {
        let conn = PgConnection::connect(conn_str)
            .await
            .map_err(|_| DomainError::ConnectFailed)?;

        Ok(Self { conn })
    }

    async fn collect_users(&mut self) -> Result<Vec<User>> {
        self.conn
            .transaction(|txn| {
                // TODO: I want to separate this part out to repository layer.
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

#[async_trait::async_trait]
trait UserRepository<DB: sqlx::Database> {
    async fn find_all(
        &self,
        tx: &mut Transaction<'_, DB>,
    ) -> std::result::Result<Vec<User>, sqlx::Error>;
}

struct PgRepo {}

impl PgRepo {
    pub async fn new() -> Self {
        Self {}
    }
}
impl UserRepository<Postgres> for PgRepo {
    fn find_all<'life0, 'life1, 'life2, 'async_trait>(
        &'life0 self,
        tx: &'life1 mut Transaction<'life2, Postgres>,
    ) -> Pin<
        Box<dyn Future<Output = std::result::Result<Vec<User>, sqlx::Error>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            sqlx::query_as("SELECT * FROM users")
                .fetch_all(&mut **tx)
                .await
        })
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
