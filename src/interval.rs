use crate::{
    error,
    handler::{chat_member::get_random_chat_member, todo::Todo},
    util,
};
use frankenstein::{AsyncApi, AsyncTelegramApi, SendMessageParamsBuilder};
use sqlx::{Pool, Postgres};
use std::env;
use tokio::time::{sleep, Duration};

pub async fn interval_actions() -> Result<(), error::LeditError> {
    let (pool, api) = util::get_pool_and_api().await;
    let sleep_duration = env::var("INTERVAL_MS")
        .unwrap_or_else(|_| {
            tracing::info!("env var `interval_MS` is not set, using default value 10000");
            "10000".to_string()
        })
        .parse::<u64>()
        .unwrap_or_else(|_| {
            tracing::warn!("failed to parse value for env var `interval_MS`, using default value 10000");
            10000
        });

    tracing::info!("interval sleep duration: {} ms", sleep_duration);

    loop {
        sleep(Duration::from_millis(sleep_duration)).await;

        match re_schedule_todos(&pool).await {
            Ok(_) => tracing::info!("re-scheduling todos done"),
            Err(err) => tracing::error!("re-scheduling todos failed, error: {}", err),
        }

        match delete_one_time_todos(&pool, &api).await {
            Ok(_) => tracing::info!("delete one-time todos done"),
            Err(err) => tracing::error!("delete one-time todos failed, error: {}", err),
        }
    }
}

async fn re_schedule_todos(pool: &Pool<Postgres>) -> Result<(), error::LeditError> {
    tracing::info!("re-schedule todos");

    let todos_to_re_schedule = sqlx::query_as!(
        Todo,
        r#"
            select *
            from todos
            where 
                interval_days is not null
                and done_by is not null
                and scheduled_for < now() - interval '1 days' * interval_days
        "#
    )
    .fetch_all(pool)
    .await?;

    if !todos_to_re_schedule.is_empty() {
        tracing::info!("amount of todos to re-schedule {:#?}", todos_to_re_schedule.len());
    }

    for todo in todos_to_re_schedule {
        let assigned_user = get_random_chat_member(todo.chat_id, pool).await?;
        sqlx::query!(
            r#"
                update todos
                set
                    done_by = null,
                    scheduled_for = now(),
                    assigned_user = $2
                where 
                    id = $1
            "#,
            todo.id,
            assigned_user
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn delete_one_time_todos(pool: &Pool<Postgres>, api: &AsyncApi) -> Result<(), error::LeditError> {
    tracing::info!("delete one-time todos");

    let todos = sqlx::query_as!(
        Todo,
        r#"
            select * from 
                todos 
            where 
                done_by is not null 
                and interval_days is null 
                and scheduled_for < now() - interval '1 day'
        "#
    )
    .fetch_all(pool)
    .await?;

    for todo in todos {
        sqlx::query!(
            r#"
                delete from 
                    todos 
                where 
                    id = $1
            "#,
            todo.id
        )
        .execute(pool)
        .await?;

        api.send_message(
            &SendMessageParamsBuilder::default()
                .chat_id(todo.chat_id)
                .text(&format!("🗑 Deleting old & done todo: {}", todo.description))
                .build()?,
        )
        .await?;
    }

    Ok(())
}
