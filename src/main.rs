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
        use sqlx::{PgConnection, PgPool};

        use crate::domain::{
            entity::user::{User, UserId},
            error::DomainError,
            repository::user_repository::UserRepository,
        };

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
        }

        #[async_trait]
        impl UserRepository for PgUserRepository {
            async fn create(&self, user: &User) -> Result<(), DomainError> {
                let mut conn = self.pool.acquire().await?;
                let result = InternalUserRepository::create(user, &mut conn).await?;
                Ok(result)
            }

            async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
                let mut conn = self.pool.acquire().await?;
                let user = InternalUserRepository::find_by_id(id, &mut conn).await?;
                Ok(user)
            }
        }

        pub(in crate::infrastructure) struct InternalUserRepository {}

        impl InternalUserRepository {
            pub(in crate::infrastructure) async fn create(
                user: &User,
                conn: &mut PgConnection,
            ) -> Result<(), DomainError> {
                sqlx::query("INSERT INTO bookshelf_user (id) VALUES ($1)")
                    .bind(user.id.as_str())
                    .execute(conn)
                    .await?;
                Ok(())
            }

            async fn find_by_id(
                id: &UserId,
                conn: &mut PgConnection,
            ) -> Result<Option<User>, DomainError> {
                let row: Option<UserRow> =
                    sqlx::query_as("SELECT * FROM bookshelf_user WHERE id = $1")
                        .bind(id.as_str())
                        .fetch_optional(conn)
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
