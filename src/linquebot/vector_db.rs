use super::db::DB_CONNECTION;
use duckdb::{params, types::ToSqlOutput};
use log::info;

pub struct VectorDB {}

/// <https://github.com/duckdb/duckdb-rs/issues/338>
trait SerializeVector {
    fn ser_to_sql(&'_ self) -> ToSqlOutput<'_>;
}

impl SerializeVector for Vec<f32> {
    fn ser_to_sql(&self) -> ToSqlOutput<'_> {
        ToSqlOutput::from(format!("{:?}", self))
    }
}

#[derive(Debug)]
pub struct VectorData {
    pub index: String,
    pub scope: String,
    pub vector: Vec<f32>,
}

#[derive(Debug)]
pub struct VectorQuery {
    pub scope: String,
    pub vector: Vec<f32>,
}

#[derive(Debug)]
pub struct VectorResult {
    pub scope: String,
    pub index: String,
    pub distance: f32,
}

const CREATE_VECTOR_DB_QUERY: &str = r#"
INSTALL vss;
LOAD vss;
SET hnsw_enable_experimental_persistence = true;
CREATE TABLE IF NOT EXISTS vector_db (
    index TEXT,
    scope TEXT,
    vector float[1024],
    PRIMARY KEY (index, scope),
    UNIQUE (index, scope)
);
"#;

const INSTALL_PG_EXT_QUERY: &str = r#"
INSTALL postgres;
LOAD postgres;
"#;

const MIGRATE_VECTOR_DB_QUERY: &str = r#"
INSERT INTO
	vector_db
SELECT
	"index",
	chat AS scope,
	vector::float[1024] AS vector
FROM
	postgres_query('db', '
SELECT
	"index",
	chat,
	vector::REAL[]
FROM
	vector_db;
')
ON CONFLICT ("index", scope) DO NOTHING;
"#;

const CREATE_VECTOR_INDEX_QUERY: &str =
    "CREATE INDEX vector_db_vector_idx ON vector_db USING HNSW (vector)";

const CREATE_SCOPE_INDEX_QUERY: &str = "CREATE INDEX vector_db_scope_idx ON vector_db (scope)";

const UPSERT_VECTOR_QUERY: &str = r#"
INSERT INTO
    vector_db (index, scope, vector)
VALUES
    ($1, $2, $3::float[1024]) ON CONFLICT (index, scope) DO
UPDATE
SET
    vector = $3::float[1024];
"#;

const SELECT_VECTOR_QUERY: &str = r#"
SELECT index,
    distance
FROM (
        SELECT index,
            array_distance(vector, $2::float[1024]) AS distance
        FROM vector_db
        WHERE scope = $1
    ) AS sub
ORDER BY distance
LIMIT 5;
"#;

fn get_idx_exists_query(index_name: &str) -> String {
    format!(
        "SELECT count()::bool FROM duckdb_indexes() WHERE table_name = 'vector_db' and index_name = '{}';",
        index_name
    )
}

impl VectorDB {
    pub async fn new() -> anyhow::Result<Self> {
        let mut db = DB_CONNECTION.get().await;
        db.execute_batch(CREATE_VECTOR_DB_QUERY)?;
        let tx = db.transaction()?;
        let vector_idx_exists =
            tx.query_row(&get_idx_exists_query("vector_db_vector_idx"), [], |rows| {
                rows.get::<usize, bool>(0)
            })?;
        if !vector_idx_exists {
            info!("Creating vector index...");
            tx.execute(CREATE_VECTOR_INDEX_QUERY, [])?;
        }
        let scope_idx_exists =
            tx.query_row(&get_idx_exists_query("vector_db_scope_idx"), [], |rows| {
                rows.get::<usize, bool>(0)
            })?;
        if !scope_idx_exists {
            info!("Creating scope index...");
            tx.execute(CREATE_SCOPE_INDEX_QUERY, [])?;
        }
        tx.commit()?;

        if let Ok(database_url) = std::env::var("VECTOR_DATABASE_URL") {
            info!("Migrating old vector database...");
            let tx = db.transaction()?;
            tx.execute_batch(INSTALL_PG_EXT_QUERY)?;
            // DuckDB does not support parameters in ATTACH DATABASE.
            // Since the database URL is trusted, we can use it directly.
            tx.execute(
                &format!("ATTACH DATABASE '{}' AS db (TYPE postgres)", database_url),
                [],
            )?;
            tx.execute_batch(MIGRATE_VECTOR_DB_QUERY)?;
            tx.commit()?;
            info!("Migration completed successfully.");
        }
        Ok(Self {})
    }

    pub async fn upsert(&self, data: VectorData) -> anyhow::Result<()> {
        let mut db = DB_CONNECTION.get().await;
        let tx = db.transaction()?;
        tx.execute(
            UPSERT_VECTOR_QUERY,
            params![data.index, data.scope, data.vector.ser_to_sql()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub async fn get(&self, data: VectorQuery) -> anyhow::Result<Vec<VectorResult>> {
        let db = DB_CONNECTION.get().await;
        let mut stmt = db.prepare(SELECT_VECTOR_QUERY)?;
        let rows = stmt
            .query_map(params![data.scope, data.vector.ser_to_sql()], |row| {
                Ok(VectorResult {
                    index: row.get(0)?,
                    scope: data.scope.clone(),
                    distance: row.get(1)?,
                })
            })?
            .map(|i| i.expect("Failed to map row"))
            .collect();
        Ok(rows)
    }
}
