use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{query, query_as};
use std::future::Future;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
struct Todo {
    id: i32,
    description: String,
    done: bool,
}

async fn new_conn(conn_str: &str) -> Result<PgPool, sqlx::Error> {
    let conn = PgPoolOptions::new()
        .max_connections(5)
        .connect(conn_str)
        .await?;

    Ok(conn)
}

async fn with_tx<Fut, T>(
    pool: &sqlx::PgPool,
    f: impl Fn(&mut sqlx::Transaction<'_, sqlx::Postgres>) -> Fut,
) -> Result<T, Box<dyn std::error::Error>>
where
    Fut: Future<Output = Result<T, Box<dyn std::error::Error>>>,
{
    let mut tx = pool.begin().await?;
    match f(&mut tx).await {
        Ok(v) => {
            tx.commit().await?;
            Ok(v)
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

async fn get_todo(
    mut executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    id: i32,
) -> Result<Option<Todo>, sqlx::Error> {
    let todo = query_as!(Todo, r#"SELECT * FROM todos WHERE id = $1"#, id)
        .fetch_optional(executor)
        .await?;

    Ok(todo)
}

async fn insert_and_verify(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    test_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    query!(
        r#"INSERT INTO todos (id, description) VALUES ($1, $2)"#,
        test_id,
        "test todo"
    )
    .execute(&mut **transaction)
    .await?;

    let _ = query!(r#"SELECT FROM todos WHERE id = $1"#, test_id)
        .execute(&mut **transaction)
        .await?;

    Ok(())
}

async fn explicit_rollback_example(
    pool: &sqlx::PgPool,
    test_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut tx = pool.begin().await?;

    insert_and_verify(&mut tx, test_id).await?;

    tx.rollback().await?;

    Ok(())
}

async fn implicit_rollback_example(
    pool: &sqlx::PgPool,
    test_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut tx = pool.begin().await?;

    insert_and_verify(&mut tx, test_id).await?;

    Ok(())
}

async fn commit_example(
    pool: &sqlx::PgPool,
    test_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut tx = pool.begin().await?;

    insert_and_verify(&mut tx, test_id).await?;

    tx.commit().await?;

    Ok(())
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn_str =
        std::env::var("DATABASE_URL").expect("Env var DATABASE_URL is required for this example");
    let pool = new_conn(&conn_str).await?;

    let test_id = 1_i32;
    let _ = query!(r#"DELETE FROM todos WHERE id = $1"#, test_id)
        .execute(&pool)
        .await?;

    explicit_rollback_example(&pool, test_id).await?;

    let inserted_todo = query!(r#"SELECT FROM todos WHERE id = $1"#, test_id)
        .fetch_one(&pool)
        .await;

    assert!(inserted_todo.is_err());

    implicit_rollback_example(&pool, test_id).await?;

    let inserted_todo = query!(r#"SELECT FROM todos WHERE id = $1"#, test_id)
        .fetch_one(&pool)
        .await;

    assert!(inserted_todo.is_err());

    commit_example(&pool, test_id).await?;

    let inserted_todo = query!(r#"SELECT FROM todos WHERE id = $1"#, test_id)
        .fetch_one(&pool)
        .await;

    assert!(inserted_todo.is_ok());

    // let todo = with_tx(&pool, |tx| get_todo(tx, test_id)).await?;

    // println!("todo: {:?}", todo);

    Ok(())
}
