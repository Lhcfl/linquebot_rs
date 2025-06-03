use log::warn;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};

pub struct VectorDB {
    pool: Pool<Postgres>,
}

pub struct VectorData {
    pub index: String,
    pub user: Option<String>,
    pub chat: String,
    pub vector: Vec<f64>,
}

const CREATE_VECTOR_DB_QUERY: &str = r#"
CREATE TABLE IF NOT EXISTS vector_db (
    id SERIAL PRIMARY KEY,
    index TEXT NULL,
    "user" TEXT NULL,
    chat TEXT NULL,
    vector vector(1024),
    UNIQUE (index, "user", chat)
)
"#;

const UPSERT_VECTOR_QUERY: &str = r#"
INSERT INTO
    vector_db (index, "user", chat, vector)
VALUES
    ($1, $2, $3, $4::vector(1024)) ON CONFLICT (index, "user", chat) DO
UPDATE
SET
    vector = $4::vector(1024);
"#;

impl VectorDB {
    async fn init() -> anyhow::Result<Self> {
        let database_url = std::env::var("VECTOR_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/linquebot".to_string());
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await?;
        sqlx::query(CREATE_VECTOR_DB_QUERY).execute(&pool).await?;
        Ok(VectorDB { pool })
    }

    pub async fn new() -> Option<Self> {
        let db = VectorDB::init().await;
        match db {
            Ok(db) => Some(db),
            Err(e) => {
                warn!("Failed to initialize VectorDB:\n{}", e);
                None
            }
        }
    }

    pub async fn upsert(&self, data: VectorData) -> anyhow::Result<()> {
        sqlx::query(UPSERT_VECTOR_QUERY)
            .bind(&data.index)
            .bind(&data.user)
            .bind(&data.chat)
            .bind(format!("{:?}", data.vector))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get(&self, data: VectorData) -> anyhow::Result<Vec<VectorData>> {
        let rows = sqlx::query(concat!(
            "SELECT vector FROM vector_db WHERE index = $1 AND \"user\" = $2 AND chat = $3 ",
            "ORDER BY vector <-> $4 LIMIT 10"
        ))
        .bind(&data.index)
        .bind(&data.user)
        .bind(&data.chat)
        .bind(&data.vector)
        .fetch_all(&self.pool)
        .await?;
        let mut result = Vec::new();
        for row in rows {
            let vector: Vec<f64> = row.get(0);
            result.push(VectorData {
                index: data.index.to_string(),
                user: data.user.clone(),
                chat: data.chat.to_string(),
                vector,
            });
        }
        Ok(result)
    }
}
