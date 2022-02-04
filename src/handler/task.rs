use crate::{error::LeditError, handler::simple_inline_keyboard};
use frankenstein::{Message, SendMessageParams, SendMessageParamsBuilder};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Postgres};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct Task {
    pub id: Uuid,
    pub chat_id: i64,
    pub description: String,
    pub interval_days: Option<i64>,
    pub deleted: bool,
}

pub async fn handle_add_task(
    title: String,
    message: &Message,
    pool: &Pool<Postgres>,
) -> Result<SendMessageParams, LeditError> {
    let task = sqlx::query_as!(
        Task,
        r#"
            insert into tasks (
                id,
                chat_id,
                description
            )
            values ( $1, $2, $3 )
            RETURNING *
        "#,
        Uuid::new_v4(),
        message.chat.id,
        &title,
    )
    .fetch_one(pool)
    .await?;

    let _buttons = simple_inline_keyboard(vec![
        ("🔁 daily".to_string(), format!("/callback {} {}", task.id, 1)),
        ("🔁 weekly".to_string(), format!("/callback {} {}", task.id, 7)),
        ("🔁 monthly".to_string(), format!("/callback {} {}", task.id, 30)),
    ]);

    let send_message_params = SendMessageParamsBuilder::default()
        .chat_id(message.chat.id)
        .text(format!("New task added:\n\n{}", title))
        // .reply_markup(buttons)
        .build()?;

    Ok(send_message_params)
}

pub async fn handle_list_tasks(message: &Message, pool: &Pool<Postgres>) -> Result<SendMessageParams, LeditError> {
    let text = get_task_list_string(message, pool, true).await?;

    let send_message_params = SendMessageParamsBuilder::default()
        .chat_id(message.chat.id)
        .text(text)
        .build()?;

    Ok(send_message_params)
}

pub async fn handle_delete_task(
    num: i64,
    message: &Message,
    pool: &Pool<Postgres>,
) -> Result<SendMessageParams, LeditError> {
    let task_to_delete = sqlx::query_as!(
        Task,
        "select * from tasks where chat_id = $1 and deleted = false order by description asc offset $2",
        message.chat.id,
        num - 1
    )
    .fetch_optional(pool)
    .await?;

    if let Some(task_to_delete) = task_to_delete {
        sqlx::query_as!(
            Task,
            r#"
                update tasks set deleted = true where id=$1
            "#,
            task_to_delete.id
        )
        .execute(pool)
        .await?;

        let mut text = format!("Deleted this task:\n\n{}\n\n", task_to_delete.description);
        text.push_str(&get_task_list_string(message, pool, false).await?);

        let send_message_params = SendMessageParamsBuilder::default()
            .chat_id(message.chat.id)
            .text(text)
            .build()?;

        Ok(send_message_params)
    } else {
        let send_message_params = SendMessageParamsBuilder::default()
            .chat_id(message.chat.id)
            .text("Task not found.")
            .build()?;

        Ok(send_message_params)
    }
}

pub async fn handle_set_task_repetition(
    task_id: Uuid,
    interval_days: i64,
    chat_id: i64,
    pool: &Pool<Postgres>,
) -> Result<SendMessageParams, LeditError> {
    let task = sqlx::query_as!(
        Task,
        r#"
            update tasks 
            set interval_days = $1
            where id=$2 and chat_id=$3
            returning *
        "#,
        interval_days,
        task_id,
        chat_id
    )
    .fetch_one(pool)
    .await?;

    let send_message_params = SendMessageParamsBuilder::default()
        .chat_id(chat_id)
        .text(format!(
            "The following task will be repeated every {} day(s):\n\n{}",
            interval_days, task.description
        ))
        .build()?;

    Ok(send_message_params)
}

async fn get_task_list_string(message: &Message, pool: &Pool<Postgres>, hint: bool) -> Result<String, LeditError> {
    let tasks = sqlx::query_as!(
        Task,
        "select * from tasks where chat_id = $1 and deleted = false order by description asc",
        message.chat.id,
    )
    .fetch_all(pool)
    .await?;

    let mut text = "List of all tasks:".to_string();
    let mut n = 1;
    for task in &tasks {
        text.push_str(&format!("\n {}. {}", n, task.description));
        n += 1;
    }

    if tasks.len() < 1 {
        text = "No task found.".to_string();
    } else if hint {
        text.push_str("\n\nHint: Use /delete 1\nto delete the first task.");
    }

    Ok(text)
}
