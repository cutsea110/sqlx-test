use sqlx::postgres::PgConnection;
use sqlx::{Connection, Transaction};

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
    pub async fn new(conn: PgConnection) -> Result<Self> {
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
trait UserRepository {
    type DB: sqlx::Database;

    async fn find_all<'a>(&self, tx: &mut Transaction<'a, Self::DB>) -> Result<Vec<User>>;
}

struct PgRepo {
    conn: PgConnection,
}
impl PgRepo {
    pub async fn new(conn_str: &str) -> Self {
        let conn = PgConnection::connect(conn_str)
            .await
            .expect("Failed to connect to DB");

        Self { conn }
    }
}
impl UserRepository for PgRepo {
    type DB = sqlx::Postgres;

    fn find_all<'a, 'life0, 'life1, 'async_trait>(
        &'life0 self,
        tx: &'life1 mut Transaction<'a, Self::DB>,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<Vec<User>>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'a: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            sqlx::query_as("SELECT * FROM users")
                .fetch_all(&mut **tx)
                .await
                .map_err(DomainError::SqlxError)
        })
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    let db_url =
        std::env::var("DATABASE_URL").expect("Env var DATABASE_URL is required for this test");
    let conn = PgConnection::connect(&db_url)
        .await
        .map_err(|_| DomainError::ConnectFailed)?;
    let mut usecase = Usecase::new(conn).await?;

    let users = usecase.collect_users().await?;

    println!("{:#?}", users);

    Ok(())
}
