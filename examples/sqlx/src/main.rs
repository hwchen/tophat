//! TODO double check... I'm using smol here, but maybe sqlx is just starting up async-std runtime
//! and using that instead.
//!
//! This is the beginning of an example for database access.
//!
//! Still needs:
//! - db creation hardcoded in.
//! - web api from tophat.

use sqlx::postgres::PgPool;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    smol::block_on(async move {
        let db_url = env::var("DATABASE_URL").expect("no db env var found");
        let pool = PgPool::connect(&db_url).await?;

        struct Country { country: Option<String>, count: Option<i64> }
        let organization = "Apple";

        let countries = sqlx::query_as!(
            Country,
            "SELECT country, COUNT(*) as count FROM users WHERE organization = $1 GROUP BY country",
            organization
        )
        .fetch_all(&pool).await?;

        //let row: (i64,) = sqlx::query_as("SELECT $1")
        //    .bind(150_i64)
        //    .fetch_one(&pool).await?;
        //assert_eq!(row.0, 150);

        for country in countries {
            println!("{:?}", country.country);
            println!("{:?}", country.count);
        }

        Ok::<_, sqlx::Error>(())
    })?;

    Ok(())
}
