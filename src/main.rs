use sqlx::postgres::PgConnection;
use sqlx::{Connection, Postgres};

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

struct UserUsecase {
    repo: Box<dyn IUserRepo>,
}
impl HaveUserRepo for UserUsecase {
    fn dao(&self) -> &dyn IUserRepo {
        &*self.repo
    }
    fn tx_run<'a, 'async_trait, F, T>(
        &'a mut self,
        f: F,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = Result<T>> + core::marker::Send + 'async_trait>,
    >
    where
        T: std::marker::Send,
        for<'c> F: FnOnce(
                &'c mut sqlx::Transaction<Postgres>,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<T>> + std::marker::Send + 'c>,
            > + std::marker::Send
            + 'a,
        'a: 'async_trait,
        F: 'async_trait,
        T: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let mut conn =
                sqlx::PgConnection::connect("postgres://postgres:postgres@localhost:5432/test")
                    .await
                    .map_err(RepositoryError::SqlxError)?;
            let mut tx = conn.begin().await.map_err(RepositoryError::SqlxError)?;

            let ret = f(&mut tx).await;

            match ret {
                Ok(v) => {
                    tx.commit().await.map_err(RepositoryError::SqlxError)?;
                    Ok(v)
                }
                Err(e) => {
                    tx.rollback().await.map_err(RepositoryError::SqlxError)?;
                    Err(e)
                }
            }
        })
    }
}
impl UserUsecase {
    pub fn new(repo: Box<dyn IUserRepo>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
trait IUserRepo {
    async fn add_user<'a>(
        &mut self,
        txn: &'a mut sqlx::Transaction<'_, Postgres>,
        name: String,
        email: String,
    ) -> Result<User>;
}
#[async_trait::async_trait]
trait HaveUserRepo {
    fn dao(&self) -> &dyn IUserRepo;
    async fn tx_run<'a, F, T>(&'a mut self, f: F) -> Result<T>
    where
        T: std::marker::Send,
        for<'c> F: FnOnce(
                &'c mut sqlx::Transaction<Postgres>,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<T>> + std::marker::Send + 'c>,
            > + std::marker::Send
            + 'a;
}

struct PgRepo {
    conn: PgConnection,
}

impl PgRepo {
    pub async fn new(conn: PgConnection) -> Result<Self> {
        Ok(Self { conn })
    }

    async fn tx_run<'a, F, T: std::marker::Send>(&'a mut self, f: F) -> Result<T>
    where
        for<'c> F: FnOnce(
                &'c mut sqlx::Transaction<Postgres>,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<T>> + std::marker::Send + 'c>,
            > + std::marker::Send
            + 'a,
    {
        let mut tx = self
            .conn
            .begin()
            .await
            .map_err(RepositoryError::SqlxError)?;

        let ret = f(&mut tx).await;

        match ret {
            Ok(v) => {
                tx.commit().await.map_err(RepositoryError::SqlxError)?;
                Ok(v)
            }
            Err(e) => {
                tx.rollback().await.map_err(RepositoryError::SqlxError)?;
                Err(e)
            }
        }
    }

    async fn test_tx_run<'a, F, T: std::marker::Send>(&'a mut self, f: F) -> Result<T>
    where
        for<'c> F: FnOnce(
                &'c mut sqlx::Transaction<Postgres>,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<T>> + std::marker::Send + 'c>,
            > + std::marker::Send
            + 'a,
    {
        let mut tx = self
            .conn
            .begin()
            .await
            .map_err(RepositoryError::SqlxError)?;

        let ret = f(&mut tx).await;
        // always rollback
        tx.rollback().await.map_err(RepositoryError::SqlxError)?;
        ret
    }

    async fn tx_test(&mut self, name: String, email: String) -> Result<User> {
        self.test_tx_run(|txn| {
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
                Ok(u)
            })
        })
        .await
    }

    // TODO: リポジトリ内はこれにしたい
    async fn _add_user<'a>(
        &mut self,
        txn: &'a mut sqlx::Transaction<'_, Postgres>,
        name: String,
        email: String,
    ) -> Result<User> {
        Box::pin(async move {
            sqlx::query_as::<_, User>("INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *")
                .bind(name)
                .bind(email)
                .fetch_one(&mut **txn)
                .await
        })
        .await
        .map_err(RepositoryError::SqlxError)
    }

    // TODO: こっちは消したい
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
        .test_tx_run(|txn| {
            Box::pin(async move {
                // insert
                let u = sqlx::query_as::<_, User>(
                    "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *",
                )
                .bind("Steve")
                .bind("steve@gmail.com")
                .fetch_one(&mut **txn)
                .await
                .unwrap();

                println!("insert: {:?}", u);

                Ok(u)
            })
        })
        .await?;

    println!("tx_test: {:?}", steve);

    Ok(())
}
