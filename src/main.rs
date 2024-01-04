use sqlx::query;

pub trait Tx<Ctx> {
    type Item;
    type Err;
    fn run(self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err>;
}

impl<Ctx, T, E, F> Tx<Ctx> for F
where
    F: FnOnce(&mut Ctx) -> Result<T, E>,
{
    type Item = T;
    type Err = E;
    fn run(self, ctx: &mut Ctx) -> Result<Self::Item, Self::Err> {
        self(ctx)
    }
}

fn map<Ctx, Tx1, F, T>(tx1: Tx1, f: F) -> impl FnOnce(&mut Ctx) -> Result<T, Tx1::Err>
where
    Tx1: Tx<Ctx>,
    F: FnOnce(Tx1::Item) -> T,
{
    move |ctx| match tx1.run(ctx) {
        Ok(x) => Ok(f(x)),
        Err(e) => Err(e),
    }
}

fn and_then<Ctx, Tx1, Tx2, F>(
    tx1: Tx1,
    f: F,
) -> impl FnOnce(&mut Ctx) -> Result<Tx2::Item, Tx1::Err>
where
    Tx1: Tx<Ctx>,
    Tx2: Tx<Ctx, Err = Tx1::Err>,
    F: FnOnce(Tx1::Item) -> Tx2,
{
    move |ctx| match tx1.run(ctx) {
        Ok(x) => f(x).run(ctx),
        Err(e) => Err(e),
    }
}

fn then<Ctx, Tx1, Tx2, F>(tx1: Tx1, f: F) -> impl FnOnce(&mut Ctx) -> Result<Tx2::Item, Tx1::Err>
where
    Tx1: Tx<Ctx>,
    Tx2: Tx<Ctx, Err = Tx1::Err>,
    F: FnOnce(Result<Tx1::Item, Tx1::Err>) -> Tx2,
{
    move |ctx| f(tx1.run(ctx)).run(ctx)
}

fn or_else<Ctx, Tx1, Tx2, F>(tx1: Tx1, f: F) -> impl FnOnce(&mut Ctx) -> Result<Tx2::Item, Tx1::Err>
where
    Tx1: Tx<Ctx>,
    Tx2: Tx<Ctx, Item = Tx1::Item, Err = Tx1::Err>,
    F: FnOnce(Tx1::Err) -> Tx2,
{
    move |ctx| match tx1.run(ctx) {
        Ok(t) => Ok(t),
        Err(e) => f(e).run(ctx),
    }
}

fn join<Ctx, Tx1, Tx2>(
    tx1: Tx1,
    tx2: Tx2,
) -> impl FnOnce(&mut Ctx) -> Result<(Tx1::Item, Tx2::Item), Tx1::Err>
where
    Tx1: Tx<Ctx>,
    Tx2: Tx<Ctx, Err = Tx1::Err>,
{
    move |ctx| match (tx1.run(ctx), tx2.run(ctx)) {
        (Ok(t), Ok(u)) => Ok((t, u)),
        (Err(e), _) | (_, Err(e)) => Err(e),
    }
}

fn join3<Ctx, Tx1, Tx2, Tx3>(
    tx1: Tx1,
    tx2: Tx2,
    tx3: Tx3,
) -> impl FnOnce(&mut Ctx) -> Result<(Tx1::Item, Tx2::Item, Tx3::Item), Tx1::Err>
where
    Tx1: Tx<Ctx>,
    Tx2: Tx<Ctx, Err = Tx1::Err>,
    Tx3: Tx<Ctx, Err = Tx1::Err>,
{
    move |ctx| match (tx1.run(ctx), tx2.run(ctx), tx3.run(ctx)) {
        (Ok(t), Ok(u), Ok(v)) => Ok((t, u, v)),
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Err(e),
    }
}

fn join4<Ctx, Tx1, Tx2, Tx3, Tx4>(
    tx1: Tx1,
    tx2: Tx2,
    tx3: Tx3,
    tx4: Tx4,
) -> impl FnOnce(&mut Ctx) -> Result<(Tx1::Item, Tx2::Item, Tx3::Item, Tx4::Item), Tx1::Err>
where
    Tx1: Tx<Ctx>,
    Tx2: Tx<Ctx, Err = Tx1::Err>,
    Tx3: Tx<Ctx, Err = Tx1::Err>,
    Tx4: Tx<Ctx, Err = Tx1::Err>,
{
    move |ctx| match (tx1.run(ctx), tx2.run(ctx), tx3.run(ctx), tx4.run(ctx)) {
        (Ok(t), Ok(u), Ok(v), Ok(w)) => Ok((t, u, v, w)),
        (Err(e), _, _, _) | (_, Err(e), _, _) | (_, _, Err(e), _) | (_, _, _, Err(e)) => Err(e),
    }
}

fn map_err<Ctx, Tx1, F, E>(tx1: Tx1, f: F) -> impl FnOnce(&mut Ctx) -> Result<Tx1::Item, E>
where
    Tx1: Tx<Ctx>,
    F: FnOnce(Tx1::Err) -> E,
{
    move |ctx| match tx1.run(ctx) {
        Ok(t) => Ok(t),
        Err(e) => Err(f(e)),
    }
}

fn try_map<Ctx, Tx1, F, T>(tx1: Tx1, f: F) -> impl FnOnce(&mut Ctx) -> Result<T, Tx1::Err>
where
    Tx1: Tx<Ctx>,
    F: FnOnce(Tx1::Item) -> Result<T, Tx1::Err>,
{
    move |ctx| match tx1.run(ctx) {
        Ok(t) => f(t),
        Err(e) => Err(e),
    }
}

fn recover<Ctx, Tx1, F>(tx1: Tx1, f: F) -> impl FnOnce(&mut Ctx) -> Result<Tx1::Item, Tx1::Err>
where
    Tx1: Tx<Ctx>,
    F: FnOnce(Tx1::Err) -> Tx1::Item,
{
    move |ctx| match tx1.run(ctx) {
        Ok(t) => Ok(t),
        Err(e) => Ok(f(e)),
    }
}

fn try_recover<Ctx, Tx1, F, E>(tx1: Tx1, f: F) -> impl FnOnce(&mut Ctx) -> Result<Tx1::Item, E>
where
    Tx1: Tx<Ctx>,
    F: FnOnce(Tx1::Err) -> Result<Tx1::Item, E>,
{
    move |ctx| match tx1.run(ctx) {
        Ok(t) => Ok(t),
        Err(e) => f(e),
    }
}

fn abort<Ctx, Tx1, F>(tx1: Tx1, f: F) -> impl FnOnce(&mut Ctx) -> Result<Tx1::Item, Tx1::Err>
where
    Tx1: Tx<Ctx>,
    F: FnOnce(Tx1::Item) -> Tx1::Err,
{
    move |ctx| match tx1.run(ctx) {
        Ok(t) => Err(f(t)),
        Err(e) => Err(e),
    }
}

fn try_abort<Ctx, Tx1, F>(tx1: Tx1, f: F) -> impl FnOnce(&mut Ctx) -> Result<Tx1::Item, Tx1::Err>
where
    Tx1: Tx<Ctx>,
    F: FnOnce(Tx1::Item) -> Result<Tx1::Item, Tx1::Err>,
{
    move |ctx| match tx1.run(ctx) {
        Ok(t) => f(t),
        Err(e) => Err(e),
    }
}

async fn insert_and_verify(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    test_id: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    query!(
        r#"INSERT INTO todos (id, description) VALUES ( $1, $2 )"#,
        test_id,
        "test todo"
    )
    // In 0.7, `Transaction` can no longer implement `Executor` directly,
    // so it must be dereferenced to the internal connection type.
    .execute(&mut **transaction)
    .await?;

    // check that inserted todo can be fetched inside the uncommitted transaction
    let _ = query!(r#"SELECT FROM todos WHERE id = $1"#, test_id)
        .fetch_one(&mut **transaction)
        .await?;

    Ok(())
}

async fn explicit_rollback_example(
    pool: &sqlx::PgPool,
    test_id: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut transaction = pool.begin().await?;

    insert_and_verify(&mut transaction, test_id).await?;

    transaction.rollback().await?;

    Ok(())
}

async fn implicit_rollback_example(
    pool: &sqlx::PgPool,
    test_id: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut transaction = pool.begin().await?;

    insert_and_verify(&mut transaction, test_id).await?;

    // no explicit rollback here but the transaction object is dropped at the end of the scope
    Ok(())
}

async fn commit_example(
    pool: &sqlx::PgPool,
    test_id: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut transaction = pool.begin().await?;

    insert_and_verify(&mut transaction, test_id).await?;

    transaction.commit().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn_str =
        std::env::var("DATABASE_URL").expect("Env var DATABASE_URL is required for this example.");
    let pool = sqlx::PgPool::connect(&conn_str).await?;

    let test_id = 1;

    // remove any old values that might be in the table already with this id from a previous run
    let _ = query!(r#"DELETE FROM todos WHERE id = $1"#, test_id)
        .execute(&pool)
        .await?;

    explicit_rollback_example(&pool, test_id).await?;

    // check that inserted todo is not visible outside the transaction after explicit rollback
    let inserted_todo = query!(r#"SELECT FROM todos WHERE id = $1"#, test_id)
        .fetch_one(&pool)
        .await;

    assert!(inserted_todo.is_err());

    implicit_rollback_example(&pool, test_id).await?;

    // check that inserted todo is not visible outside the transaction after implicit rollback
    let inserted_todo = query!(r#"SELECT FROM todos WHERE id = $1"#, test_id)
        .fetch_one(&pool)
        .await;

    assert!(inserted_todo.is_err());

    commit_example(&pool, test_id).await?;

    // check that inserted todo is visible outside the transaction after commit
    let inserted_todo = query!(r#"SELECT FROM todos WHERE id = $1"#, test_id)
        .fetch_one(&pool)
        .await;

    assert!(inserted_todo.is_ok());

    Ok(())
}
