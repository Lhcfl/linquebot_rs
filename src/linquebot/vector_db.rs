use log::warn;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};

pub struct VectorDB {
    pool: Pool<Postgres>,
}

#[derive(Debug)]
pub struct VectorData {
    pub index: String,
    pub user: Option<String>,
    pub chat: String,
    pub vector: Vec<f64>,
}

#[derive(Debug)]
pub struct VectorQuery {
    pub user: Option<String>,
    pub chat: String,
    pub vector: Vec<f64>,
}

#[derive(Debug)]
pub struct VectorResult {
    pub user: Option<String>,
    pub chat: String,
    pub index: String,
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

const CREATE_VECTOR_INDEX_QUERY: &str = r#"
DO $$
BEGIN IF NOT EXISTS (
    SELECT
        1
    FROM
        pg_indexes
    WHERE
        schemaname = 'public'
        AND tablename = 'vector_db'
        AND indexname = 'vector_db_vector_idx'
) THEN CREATE INDEX vector_db_vector_idx ON vector_db USING vectors (vector vector_l2_ops);
END IF;
END$$;
"#;

const UPSERT_VECTOR_QUERY: &str = r#"
INSERT INTO
    vector_db (index, "user", chat, vector)
VALUES
    ($1, $2, $3, $4::vector) ON CONFLICT (index, "user", chat) DO
UPDATE
SET
    vector = $4::vector;
"#;

const SELECT_VECTOR_QUERY: &str = r#"
SELECT index
FROM vector_db
WHERE chat = $1
    AND "user" IS NOT DISTINCT FROM $2
ORDER BY vector <-> $3::vector
LIMIT 10;
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
        sqlx::query(CREATE_VECTOR_INDEX_QUERY)
            .execute(&pool)
            .await?;
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

    pub async fn get(&self, data: VectorQuery) -> anyhow::Result<Vec<VectorResult>> {
        let rows = sqlx::query(SELECT_VECTOR_QUERY)
            .bind(&data.chat)
            .bind(&data.user)
            .bind(format!("{:?}", data.vector))
            .fetch_all(&self.pool)
            .await?;
        let mut result = Vec::new();
        for row in rows {
            let index: String = row.get(0);
            result.push(VectorResult {
                index,
                user: data.user.clone(),
                chat: data.chat.to_string(),
            });
        }
        Ok(result)
    }
}
