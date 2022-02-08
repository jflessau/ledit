use crate::{
    error,
    handler::{chat_member::get_random_chat_member, todo::Todo},
    util,
};
use sqlx::{Pool, Postgres};
use tokio::time::{sleep, Duration};

pub async fn interval_actions() -> Result<(), error::LeditError> {
    let (pool, _) = util::get_pool_and_api().await;

    loop {
        sleep(Duration::from_millis(1000)).await;

        match re_schedule_todos(&pool).await {
            Ok(_) => tracing::info!("re-scheduling todos done"),
            Err(err) => tracing::error!("re-scheduling todos failed, error: {}", err),
        }

        match delete_one_time_todos(&pool).await {
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

async fn delete_one_time_todos(pool: &Pool<Postgres>) -> Result<(), error::LeditError> {
    tracing::info!("delete one-time todos");

    sqlx::query!(
        r#"
            delete from 
                todos 
            where 
                done_by is not null 
                and interval_days is null 
                and scheduled_for < now() - interval '1 day'
        "#
    )
    .execute(pool)
    .await?;

    Ok(())
}
