pub mod domain {
    pub mod error {
        use thiserror::Error;

        #[derive(Debug, Clone, PartialEq, Eq, Error)]
        pub enum DomainError {
            #[error("unexpected error: {0}")]
            Unexpected(String),
        }

        impl From<sqlx::Error> for DomainError {
            fn from(err: sqlx::Error) -> Self {
                Self::Unexpected(err.to_string())
            }
        }
    }

    pub mod entity {
        pub mod user {
            use crate::domain::error;

            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct UserId {
                value: String,
            }

            impl UserId {
                pub fn new(id: String) -> Result<Self, error::DomainError> {
                    let user = Self { value: id };
                    // validate
                    if user.value.is_empty() {
                        return Err(error::DomainError::Unexpected(
                            "user id must not be empty".to_string(),
                        ));
                    }
                    Ok(user)
                }

                pub fn as_str(&self) -> &str {
                    &self.value
                }

                pub fn into_string(self) -> String {
                    self.value
                }
            }

            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct User {
                pub id: UserId,
            }

            impl User {
                pub fn new(id: UserId) -> User {
                    User { id }
                }
            }
        }
    }

    pub mod repository {
        pub mod user_repository {
            use async_trait::async_trait;

            use crate::domain::{
                entity::user::{User, UserId},
                error::DomainError,
            };

            #[async_trait]
            pub trait UserRepository: Send + Sync + 'static {
                async fn create(&self, user: &User) -> Result<(), DomainError>;
                async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError>;
            }
        }
    }
}

pub mod infrastructure {
    pub mod pg_db {
        use async_trait::async_trait;
        use sqlx::{PgPool, Transaction};
        use std::future::Future;

        use crate::domain::{
            entity::user::{User, UserId},
            error::DomainError,
            repository::user_repository::UserRepository,
        };

        type PgTransaction<'a> = Transaction<'a, sqlx::Postgres>;

        #[derive(sqlx::FromRow)]
        struct UserRow {
            id: String,
        }

        #[derive(Debug, Clone)]
        pub struct PgUserRepository {
            pool: PgPool,
        }

        impl PgUserRepository {
            pub fn new(pool: PgPool) -> Self {
                Self { pool }
            }

            async fn with_transaction<T, F, Fut>(&self, f: F) -> Result<T, DomainError>
            where
                F: FnOnce(&mut PgTransaction<'_>) -> Fut,
                Fut: Future<Output = Result<T, DomainError>>,
            {
                let mut tx = self.pool.begin().await?;
                match f(&mut tx).await {
                    Ok(v) => {
                        tx.commit().await?;
                        Ok(v)
                    }
                    Err(err) => {
                        tx.rollback().await?;
                        Err(err)
                    }
                }
            }
        }

        #[async_trait]
        impl UserRepository for PgUserRepository {
            async fn create(&self, user: &User) -> Result<(), DomainError> {
                let mut tx = self.pool.begin().await?;
                match InternalUserRepository::create(user, &mut tx).await {
                    Ok(result) => {
                        tx.commit().await?;
                        Ok(result)
                    }
                    Err(err) => {
                        tx.rollback().await?;
                        Err(err)
                    }
                }
            }

            async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
                let mut tx = self.pool.begin().await?;
                match InternalUserRepository::find_by_id(id, &mut tx).await {
                    Ok(result) => {
                        tx.commit().await?;
                        Ok(result)
                    }
                    Err(err) => {
                        tx.rollback().await?;
                        Err(err)
                    }
                }
            }
        }

        pub(in crate::infrastructure) struct InternalUserRepository {}

        impl InternalUserRepository {
            pub(in crate::infrastructure) async fn create(
                user: &User,
                tx: &mut PgTransaction<'_>,
            ) -> Result<(), DomainError> {
                sqlx::query("INSERT INTO bookshelf_user (id) VALUES ($1)")
                    .bind(user.id.as_str())
                    .execute(&mut **tx)
                    .await?;
                Ok(())
            }

            async fn find_by_id(
                id: &UserId,
                tx: &mut PgTransaction<'_>,
            ) -> Result<Option<User>, DomainError> {
                let row: Option<UserRow> =
                    sqlx::query_as("SELECT * FROM bookshelf_user WHERE id = $1")
                        .bind(id.as_str())
                        .fetch_optional(&mut **tx)
                        .await?;

                let id = row.map(|row| UserId::new(row.id)).transpose()?;
                Ok(id.map(|id| User::new(id)))
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;
            use sqlx::postgres::PgPoolOptions;

            #[async_std::test]
            async fn test_user_repository() -> anyhow::Result<()> {
                let db_url = std::env::var("DATABASE_URL")
                    .expect("Env var DATABASE_URL is required. for this test");
                let pool = PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&db_url)
                    .await?;
                let mut tx = pool.begin().await?;

                let id = UserId::new(String::from("foo"))?;
                let user = User::new(id.clone());

                let fetched_user = InternalUserRepository::find_by_id(&id, &mut tx).await?;
                assert!(fetched_user.is_none());

                InternalUserRepository::create(&user, &mut tx).await?;

                let fetched_user = InternalUserRepository::find_by_id(&id, &mut tx).await?;
                assert_eq!(fetched_user, Some(user));

                tx.rollback().await?;
                Ok(())
            }
        }
    }
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
