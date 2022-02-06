use crate::{
    error,
    get_pool_and_api,
    handler::task::{get_random_chat_member, Task},
};
use tokio::time::{sleep, Duration};

pub async fn re_schedule_tasks() -> Result<(), error::LeditError> {
    let (pool, _) = get_pool_and_api().await;
    loop {
        sleep(Duration::from_millis(100000)).await;

        // re-schedule tasks
        let tasks_to_re_schedule = sqlx::query_as!(
            Task,
            r#"
                select *
                from tasks
                where 
                    interval_days is not null
                    and done_by is not null
                    and scheduled_for < now() - interval '1 days' * interval_days
            "#
        )
        .fetch_all(&pool)
        .await?;

        if !tasks_to_re_schedule.is_empty() {
            println!("reschedule {} tasks", tasks_to_re_schedule.len());
        }

        for task in tasks_to_re_schedule {
            let assigned_user = get_random_chat_member(task.chat_id, &pool).await?;
            sqlx::query!(
                r#"
                    update tasks
                    set
                        done_by = null,
                        scheduled_for = now(),
                        assigned_user = $1
                    where 
                        id = $2
                "#,
                task.id,
                assigned_user
            )
            .execute(&pool)
            .await?;
        }

        // delete one-time tasks that are done
        sqlx::query!(
            r#"
                delete from 
                    tasks 
                where 
                    done_by is not null 
                    and interval_days is null 
                    and scheduled_for < now() - interval '1 day'
            "#
        )
        .execute(&pool)
        .await?;
    }
}
