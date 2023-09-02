use sqlx::postgres::PgConnection;
use sqlx::Connection;

#[derive(Debug)]
enum RepositoryError {
    ConnectFailed,
    SqlxError(sqlx::Error),
}

type Result<T> = std::result::Result<T, RepositoryError>;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
struct User {
    id: i32,
    name: String,
    email: String,
}

struct PgRepo {
    conn: PgConnection,
}

impl PgRepo {
    pub async fn new(conn: PgConnection) -> Result<Self> {
        Ok(Self { conn })
    }

    async fn tx_test(&mut self, name: String, email: String) -> Result<User> {
        self.conn
            .transaction(|txn| {
                Box::pin(async move {
                    // insert
                    let u = sqlx::query_as::<_, User>(
                        "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *",
                    )
                    .bind(name)
                    .bind(email)
                    .fetch_one(&mut **txn)
                    .await
                    .unwrap();
                    println!("insert: {:?}", u);

                    // select
                    let u = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
                        .bind(u.id)
                        .fetch_one(&mut **txn)
                        .await
                        .unwrap();

                    println!("select: {:?}", u);

                    // delete
                    let _ =
                        sqlx::query_as::<_, User>("DELETE FROM users WHERE id = $1 RETURNING *")
                            .bind(u.id)
                            .fetch_one(&mut **txn)
                            .await;

                    Ok(u)
                })
            })
            .await
            .map_err(RepositoryError::SqlxError)
    }

    // テスト想定なのでつねに rollback する
    async fn tx_test_low_level(&mut self, name: String, email: String) -> Result<User> {
        let mut txn = self
            .conn
            .begin()
            .await
            .map_err(RepositoryError::SqlxError)?;
        Box::pin(async move {
            // insert
            let u = sqlx::query_as::<_, User>(
                "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *",
            )
            .bind(name)
            .bind(email)
            .fetch_one(&mut *txn)
            .await;

            println!("insert: {:?}", u);

            match u {
                Ok(u) => {
                    // select
                    println!("succeed to insert: {:?}", u);

                    let u = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
                        .bind(u.id)
                        .fetch_one(&mut *txn)
                        .await
                        .unwrap();
                    println!("select: {:?}", u);

                    txn.rollback().await.map_err(RepositoryError::SqlxError)?;

                    return Ok(u);
                }
                Err(e) => {
                    println!("insert failed: {:?}", e);

                    txn.rollback().await.map_err(RepositoryError::SqlxError)?;
                    return Err(RepositoryError::SqlxError(e));
                }
            }
        })
        .await
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
            .map_err(RepositoryError::SqlxError)
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
            .map_err(RepositoryError::SqlxError)
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
            .map_err(RepositoryError::SqlxError)
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
            .map_err(RepositoryError::SqlxError)
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
            .map_err(RepositoryError::SqlxError)
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    let db_url =
        std::env::var("DATABASE_URL").expect("Env var DATABASE_URL is required for this test");
    let conn = PgConnection::connect(&db_url)
        .await
        .map_err(|_| RepositoryError::ConnectFailed)?;
    let mut user_db = PgRepo::new(conn).await?;

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

    let kate = user_db
        .tx_test("Kate".into(), "kate@gmail.com".into())
        .await?;

    println!("tx_test: {:?}", kate);

    let steve = user_db
        .tx_test_low_level("Steve".into(), "steve@gmail.com".into())
        .await?;

    println!("tx_test_low_level: {:?}", steve);

    Ok(())
}
