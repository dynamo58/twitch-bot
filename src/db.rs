use sqlx::{Connection, Sqlite, SqliteConnection};

// generate tables for Markov chains
pub async fn index_tables_for_markov(db: &mut SqliteConnection)
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
        .execute(&mut *db)
        .await?
        .split(" ").collect::<&str>();
    
    let sql = r#"
        SELECT 
            COUNT
        FROM 
            $1;
    "#;
    
    for table_name in tables {
        // TODO: remake `try_create_table` such that it takes a literal, not a 
        try_create_table(&pool, channel).await?;

        let mut count = sqlx::query::<Sqlite>(sql)
            .bind(table_name)
            .execute(&mut *db)
            .await?
            .parse::(u32).unwrap();

        let mut offset = 0;
        while count != 0 {
            let messages = sqlx::query::<Sqlite>("SELECT message FROM $1 LIMIT=100 OFFSET $2;")
                .bind(table_name)
                .bind(offset)
                .execute(&mut *db)
                .await?
                .parse::(Vec<&str>).unwrap();

                for msg in messages {
                    let words = msg.split(" ");

                    for idx in words.len() {
                        let pred = match idx {
                            0 => "",
                            _ => words[idx - 1],
                        };
                        let word = words[idx];
                        let succ = match idx {
                            words.len() - 1 => "",
                            _ => words[idx + 1];
                        }

                        // sqlx::query::<Sqlite>("INSERT INTO  FROM $1 LIMIT=100 OFFSET $2;")
                        //     .bind(table_name)
                        //     .bind(offset)
                        //     .execute(&mut *db)
                        //     .await?
                        //     .parse::(Vec<&str>).unwrap();


                        // TODO 
                        // tohle je absolutní clusterfuck
                        // https://www.sqlitetutorial.net/sqlite-attach-database/

                    }

                }
            
                
                if messages.len() < 100 {
                    break;
                }

                offset += 100;
                count -= 100;
        }
        
        
    }
}