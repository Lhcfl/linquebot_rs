use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};

pub struct VectorDB {
    pool: Pool<Postgres>,
}

#[derive(Debug)]
pub struct VectorData {
    pub index: String,
    pub user: Option<String>,
    pub chat: String,
    pub vector: Vec<f32>,
}

#[derive(Debug)]
pub struct VectorQuery {
    pub user: Option<String>,
    pub chat: String,
    pub vector: Vec<f32>,
}

#[derive(Debug)]
pub struct VectorResult {
    pub user: Option<String>,
    pub chat: String,
    pub index: String,
    pub distance: f32,
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
) THEN CREATE INDEX vector_db_vector_idx ON vector_db USING vchordrq (vector vector_l2_ops) WITH 
(options = 'residual_quantization = true
[build.internal]
lists=[]');
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
SELECT index,
    distance
FROM (
        SELECT index,
            (vector <-> $3::vector)::FLOAT4 AS distance
        FROM vector_db
        WHERE chat = $1
            AND "user" IS NOT DISTINCT
        FROM $2
    ) AS sub
ORDER BY distance
LIMIT 5;
"#;

impl VectorDB {
    pub async fn new() -> anyhow::Result<Self> {
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
            result.push(VectorResult {
                index: row.get(0),
                user: data.user.clone(),
                chat: data.chat.clone(),
                distance: row.get(1),
            });
        }
        Ok(result)
    }
}
