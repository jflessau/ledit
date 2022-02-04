use crate::{
    error::LeditError,
    handler::{chat_member::ChatMember, task::Task},
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use frankenstein::{SendMessageParams, SendMessageParamsBuilder};
use itertools::Itertools;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Postgres};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct Execution {
    pub id: Uuid,
    pub task_id: Uuid,
    pub assigned_user: Uuid,
    pub scheduled_for: NaiveDate,
    pub completed_at: Option<DateTime<Utc>>,
    pub completed_by: Option<Uuid>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TodoListItem {
    chat_member_id: Uuid,
    username: String,
    task_id: Uuid,
    description: String,
}

pub async fn handle_get_todos(chat_id: i64, pool: &Pool<Postgres>) -> Result<SendMessageParams, LeditError> {
    println!("get todos");

    let today = Utc::today();
    let today = NaiveDate::from_ymd(today.year(), today.month(), today.day());

    // get actionable tasks with missing execution
    let actionable_tasks_without_execution = sqlx::query_as!(
        Task,
        r#"
            select 
                t.id,
                t.chat_id,
                t.description,
                t.interval_days,
                t.deleted
            from 
                tasks as t
            left outer join 
                executions as e on e.task_id = t.id
            where 
                t.chat_id = $1
                and t.deleted = false
                and e.id is null
            
        "#,
        chat_id
    )
    .fetch_all(pool)
    .await?;

    println!(
        "actionable_tasks_without_execution: {:#?}",
        actionable_tasks_without_execution
            .clone()
            .into_iter()
            .map(|v| v.description)
            .collect::<Vec<String>>()
    );

    // get chat_members
    let chat_member_ids = sqlx::query!(
        r#"
            select 
                id
            from chat_members where chat_id = $1
        "#,
        chat_id
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|v| v.id)
    .collect::<Vec<Uuid>>();

    // create execution for actionable tasks without execution
    let new_executions = actionable_tasks_without_execution
        .into_iter()
        .map(|task| {
            if let Some(assigned_user) = chat_member_ids.choose(&mut rand::thread_rng()) {
                Some(Execution {
                    id: Uuid::new_v4(),
                    task_id: task.id,
                    assigned_user: *assigned_user,
                    scheduled_for: today,
                    completed_at: None,
                    completed_by: None,
                })
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<Execution>>();

    

    let send_message_params = SendMessageParamsBuilder::default()
        .chat_id(chat_id)
        .text("lorem")
        .build()?;

    Ok(send_message_params)
}
