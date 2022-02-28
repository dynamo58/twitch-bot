use sqlx::{Connection, Sqlite, SqliteConnection};

// generate tables for Markov chains
pub async fn index_tables_for_markov(client: &mut SqliteConnection)
-> anyhow::Result<()> {
    let sql = r#"
        SELECT 
            name
        FROM 
            sqlite_schema
        WHERE 
            type ='table' AND 
            name NOT LIKE 'sqlite_%';
    "#;
    let count = sqlx::query::<Sqlite>()
        .bind(&self.code)
        .bind(self.id)
        .execute(&mut *client)
        .await?;
}