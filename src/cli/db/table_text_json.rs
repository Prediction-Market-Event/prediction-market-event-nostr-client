#[macro_export]
macro_rules! impl_table {
    ($t:ty, $sql_table_name:expr, $value:ty) => {
        impl $t {
            #[allow(unused)]
            const SQL_TABLE_NAME: &'static str = $sql_table_name;

            #[allow(unused)]
            async fn init_table(db_pool: &Pool<Sqlite>) -> Result<()> {
                let raw = format!(
                    "
                        CREATE TABLE IF NOT EXISTS {} (
                            k TEXT PRIMARY KEY,
                            v TEXT NOT NULL
                        )
                    ",
                    Self::SQL_TABLE_NAME
                );

                sqlx::query(&raw).execute(db_pool).await?;

                Ok(())
            }

            #[allow(unused)]
            async fn put<K: Into<String>>(context: &Context, key: K, value: &$value) -> Result<()> {
                let raw = format!(
                    "
                        INSERT INTO {} (k, v)
                        VALUES (?, ?)
                        ON CONFLICT(k) DO UPDATE SET v = excluded.v
                    ",
                    Self::SQL_TABLE_NAME
                );

                let value_json = serde_json::to_string(value)?;
                sqlx::query(&raw)
                    .bind::<String>(key.into())
                    .bind::<String>(value_json)
                    .execute(&context.db_pool)
                    .await?;

                Ok(())
            }

            #[allow(unused)]
            async fn get<K: Into<String>>(context: &Context, key: K) -> Result<Option<$value>> {
                let raw = format!(
                    "
                        SELECT v
                        FROM {}
                        WHERE k = ?
                    ",
                    Self::SQL_TABLE_NAME
                );

                let row_option = sqlx::query(&raw)
                    .bind::<String>(key.into())
                    .fetch_optional(&context.db_pool)
                    .await?;

                let Some(row) = row_option else {
                    return Ok(None);
                };
                let value: $value = serde_json::from_str(row.get(0))?;

                Ok(Some(value))
            }

            #[allow(unused)]
            async fn get_all(context: &Context) -> Result<Vec<(String, $value)>> {
                let raw = format!(
                    "
                        SELECT k, v
                        FROM {}
                    ",
                    Self::SQL_TABLE_NAME
                );

                let rows = sqlx::query(&raw).fetch_all(&context.db_pool).await?;
                let mut v = Vec::new();
                for row in rows {
                    let key = row.get::<String, _>(0);
                    let value = serde_json::from_str(row.get(1))?;
                    v.push((key, value));
                }

                Ok(v)
            }

            #[allow(unused)]
            async fn delete<K: Into<String>>(context: &Context, key: K) -> Result<()> {
                let raw = format!(
                    "
                        DELETE FROM {}
                        WHERE k = ?
                    ",
                    Self::SQL_TABLE_NAME
                );

                sqlx::query(&raw)
                    .bind::<String>(key.into())
                    .execute(&context.db_pool)
                    .await?;

                Ok(())
            }

            #[allow(unused)]
            async fn delete_all(context: &Context) -> Result<()> {
                let raw = format!(
                    "
                        DELETE FROM {}
                    ",
                    Self::SQL_TABLE_NAME
                );

                sqlx::query(&raw).execute(&context.db_pool).await?;

                Ok(())
            }
        }
    };
}

pub use impl_table;
