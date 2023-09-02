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

    async fn get_user(&mut self, id: i32) -> Result<Option<User>> {
        self.conn
            .transaction(|txn| {
                Box::pin(async move {
                    sqlx::query_as("SELECT * FROM users WHERE id = $1")
                        .bind(id)
                        .fetch_optional(&mut **txn)
                        .await
                })
            })
            .await
            .map_err(DomainError::SqlxError)
    }

    async fn modify_user(&mut self, id: i32, name: String, email: String) -> Result<User> {
        self.conn
            .transaction(|txn| {
                Box::pin(async move {
                    sqlx::query_as::<_, User>(
                        "UPDATE users SET name = $2, email = $3 WHERE id = $1 RETURNING *",
                    )
                    .bind(id)
                    .bind(name)
                    .bind(email)
                    .fetch_one(&mut **txn)
                    .await
                })
            })
            .await
            .map_err(DomainError::SqlxError)
    }

    async fn delete_user(&mut self, id: i32) -> Result<User> {
        self.conn
            .transaction(|txn| {
                Box::pin(async move {
                    sqlx::query_as::<_, User>("DELETE FROM users WHERE id = $1 RETURNING *")
                        .bind(id)
                        .fetch_one(&mut **txn)
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

    let john = user_db
        .add_user("John".into(), "john@google.com".into())
        .await?;

    println!("create: {:?}", john);

    let users = user_db.collect_users().await?;

    println!("list:{:#?}", users);

    let opt_john = user_db.get_user(john.id).await?;

    println!("get: {:#?}", opt_john);

    let opt_john = user_db
        .modify_user(john.id, "John Doe".into(), "d.john@gmail.com".into())
        .await?;

    println!("update: {:?}", opt_john);

    let _ = user_db.delete_user(john.id).await?;

    let opt_john = user_db.get_user(john.id).await?;

    println!("delete: {:?}", opt_john);

    Ok(())
}
