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

    let tables = sqlx::query::<Sqlite>(sql)
        .execute(&mut *client)
        .await?
        .split(" ").collect::<&str>();
    
    let sql = r#"
        SELECT 
            COUNT
        FROM 
            $1;
    "#;
    
    for table_name in tables {
        let mut count = sqlx::query::<Sqlite>(sql)
            .bind(table_name)
            .execute(&mut *client)
            .await?
            .parse::(u32).unwrap();

        while count != 0 {
            let 
        }
        
        
    }
}