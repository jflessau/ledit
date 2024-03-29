use chrono::{Datelike, NaiveDate, Utc};
use frankenstein::AsyncApi;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::env;

pub async fn get_pool_and_api() -> (Pool<Postgres>, AsyncApi) {
    let token = env::var("TOKEN").expect("missing TOKEN env var");
    let api = AsyncApi::new(&token);
    let database_url = env::var("DATABASE_URL").expect("missing DATABASE_URL env var");
    let pool = PgPoolOptions::new()
        .max_connections(16)
        .connect(&database_url)
        .await
        .expect("failed to get connection pool");

    (pool, api)
}

pub fn today() -> NaiveDate {
    let today = Utc::now();
    NaiveDate::from_ymd_opt(today.year(), today.month(), today.day()).expect("invalid date")
}
