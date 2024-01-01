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

fn map<Ctx, T, F, G>(f: F, g: G) -> impl FnOnce(&mut Ctx) -> Result<T, F::Err>
where
    F: Tx<Ctx>,
    G: FnOnce(F::Item) -> T,
{
    move |ctx| match f.run(ctx) {
        Ok(x) => Ok(g(x)),
        Err(e) => Err(e),
    }
}

fn and_then<Ctx, F, G, B>(f: F, g: G) -> impl FnOnce(&mut Ctx) -> Result<B::Item, F::Err>
where
    F: Tx<Ctx>,
    B: Tx<Ctx, Err = F::Err>,
    G: FnOnce(F::Item) -> B,
{
    move |ctx| match f.run(ctx) {
        Ok(x) => g(x).run(ctx),
        Err(e) => Err(e),
    }
}

fn then<Ctx, F, G, B>(f: F, g: G) -> impl FnOnce(&mut Ctx) -> Result<B::Item, F::Err>
where
    F: Tx<Ctx>,
    B: Tx<Ctx, Err = F::Err>,
    G: FnOnce(Result<F::Item, F::Err>) -> B,
{
    move |ctx| g(f.run(ctx)).run(ctx)
}

fn or_else<Ctx, F, G, B>(f: F, g: G) -> impl FnOnce(&mut Ctx) -> Result<B::Item, F::Err>
where
    F: Tx<Ctx>,
    B: Tx<Ctx, Item = F::Item, Err = F::Err>,
    G: FnOnce(F::Err) -> B,
{
    move |ctx| match f.run(ctx) {
        Ok(t) => Ok(t),
        Err(e) => g(e).run(ctx),
    }
}

fn join<Ctx, F, G>(f: F, g: G) -> impl FnOnce(&mut Ctx) -> Result<(F::Item, G::Item), F::Err>
where
    F: Tx<Ctx>,
    G: Tx<Ctx, Err = F::Err>,
{
    move |ctx| match (f.run(ctx), g.run(ctx)) {
        (Ok(t), Ok(u)) => Ok((t, u)),
        (Err(e), _) | (_, Err(e)) => Err(e),
    }
}

fn join3<Ctx, E, F, G, H>(
    f: F,
    g: G,
    h: H,
) -> impl FnOnce(&mut Ctx) -> Result<(F::Item, G::Item, H::Item), F::Err>
where
    F: Tx<Ctx, Err = E>,
    G: Tx<Ctx, Err = F::Err>,
    H: Tx<Ctx, Err = F::Err>,
{
    move |ctx| match (f.run(ctx), g.run(ctx), h.run(ctx)) {
        (Ok(t), Ok(u), Ok(v)) => Ok((t, u, v)),
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Err(e),
    }
}

fn join4<Ctx, F, G, H, I>(
    f: F,
    g: G,
    h: H,
    i: I,
) -> impl FnOnce(&mut Ctx) -> Result<(F::Item, G::Item, H::Item, I::Item), F::Err>
where
    F: Tx<Ctx>,
    G: Tx<Ctx, Err = F::Err>,
    H: Tx<Ctx, Err = F::Err>,
    I: Tx<Ctx, Err = F::Err>,
{
    move |ctx| match (f.run(ctx), g.run(ctx), h.run(ctx), i.run(ctx)) {
        (Ok(t), Ok(u), Ok(v), Ok(w)) => Ok((t, u, v, w)),
        (Err(e), _, _, _) | (_, Err(e), _, _) | (_, _, Err(e), _) | (_, _, _, Err(e)) => Err(e),
    }
}

fn map_err<Ctx, E, F, G>(f: F, g: G) -> impl FnOnce(&mut Ctx) -> Result<F::Item, E>
where
    F: Tx<Ctx>,
    G: FnOnce(F::Err) -> E,
{
    move |ctx| match f.run(ctx) {
        Ok(t) => Ok(t),
        Err(e) => Err(g(e)),
    }
}

fn try_map<Ctx, T, F, G>(f: F, g: G) -> impl FnOnce(&mut Ctx) -> Result<T, F::Err>
where
    F: Tx<Ctx>,
    G: FnOnce(F::Item) -> Result<T, F::Err>,
{
    move |ctx| match f.run(ctx) {
        Ok(t) => g(t),
        Err(e) => Err(e),
    }
}

fn recover<Ctx, F, G>(f: F, g: G) -> impl FnOnce(&mut Ctx) -> Result<F::Item, F::Err>
where
    F: Tx<Ctx>,
    G: FnOnce(F::Err) -> F::Item,
{
    move |ctx| match f.run(ctx) {
        Ok(t) => Ok(t),
        Err(e) => Ok(g(e)),
    }
}

fn try_recovery<Ctx, E, F, G>(f: F, g: G) -> impl FnOnce(&mut Ctx) -> Result<F::Item, E>
where
    F: Tx<Ctx>,
    G: FnOnce(F::Err) -> Result<F::Item, E>,
{
    move |ctx| match f.run(ctx) {
        Ok(t) => Ok(t),
        Err(e) => g(e),
    }
}

fn abort<Ctx, F, G>(f: F, g: G) -> impl FnOnce(&mut Ctx) -> Result<F::Item, F::Err>
where
    F: Tx<Ctx>,
    G: FnOnce(F::Item) -> F::Err,
{
    move |ctx| match f.run(ctx) {
        Ok(t) => Err(g(t)),
        Err(e) => Err(e),
    }
}

fn try_abort<Ctx, F, G>(f: F, g: G) -> impl FnOnce(&mut Ctx) -> Result<F::Item, F::Err>
where
    F: Tx<Ctx>,
    G: FnOnce(F::Item) -> Result<F::Item, F::Err>,
{
    move |ctx| match f.run(ctx) {
        Ok(t) => g(t),
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
