use crate::{error::LeditError, handler::today};
use chrono::NaiveDate;
use frankenstein::{objects::User, Message, SendMessageParams, SendMessageParamsBuilder};
use itertools::Itertools;
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Postgres};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct Task {
    pub id: Uuid,
    pub chat_id: i64,
    pub description: String,

    pub interval_days: Option<i64>,
    pub assigned_user: Uuid,
    pub scheduled_for: NaiveDate,
    pub done_by: Option<Uuid>,
}

pub async fn handle_add_task(
    title: String,
    mut interval_days: Option<i64>,
    message: &Message,
    pool: &Pool<Postgres>,
) -> Result<SendMessageParams, LeditError> {
    if let Some(v) = interval_days {
        if !(1..=999).contains(&v) {
            interval_days = None
        }
    };

    let users = sqlx::query!(r#"select id from chat_members where chat_id = $1"#, message.chat.id,)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|v| v.id)
        .collect::<Vec<Uuid>>();

    let mut rng: StdRng = SeedableRng::from_entropy();
    let n = rng.gen_range(0..users.len());
    println!("n: {}", n);
    if let Some(assigned_user) = users.get(n) {
        let task = sqlx::query_as!(
            Task,
            r#"
                insert into tasks (
                    id,
                    chat_id,
                    description,
                    
                    interval_days,
                    assigned_user

                )
                values ( $1, $2, $3, $4, $5 )
                RETURNING *
            "#,
            Uuid::new_v4(),
            message.chat.id,
            &title,
            interval_days,
            assigned_user,
        )
        .fetch_one(pool)
        .await?;

        let send_message_params = SendMessageParamsBuilder::default()
            .chat_id(message.chat.id)
            .text(format!("New task added:\n\n{}", task.description))
            .build()?;

        Ok(send_message_params)
    } else {
        let send_message_params = SendMessageParamsBuilder::default()
            .chat_id(message.chat.id)
            .text("Failed to assign task.".to_string())
            .build()?;

        Ok(send_message_params)
    }
}

pub async fn handle_list_tasks(message: &Message, pool: &Pool<Postgres>) -> Result<SendMessageParams, LeditError> {
    let mut text = get_task_list_string(message, pool).await?;

    text.push_str("\n\n\n");
    text.push_str(&get_todos(message.chat.id, pool).await?);

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
        "select * from tasks where chat_id = $1 order by description asc offset $2",
        message.chat.id,
        num - 1
    )
    .fetch_optional(pool)
    .await?;

    if let Some(task_to_delete) = task_to_delete {
        sqlx::query!(
            r#"
                delete from tasks where id = $1
            "#,
            task_to_delete.id
        )
        .execute(pool)
        .await?;

        let mut text = format!("Deleted this task:\n\n{}\n\n", task_to_delete.description);
        text.push_str(&get_task_list_string(message, pool).await?);

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

pub async fn handle_check_task(
    num: i64,
    message: &Message,
    pool: &Pool<Postgres>,
) -> Result<SendMessageParams, LeditError> {
    if let Some(User { id, .. }) = message.from.as_ref() {
        let user = sqlx::query!("select id from chat_members where telegram_user_id = $1", *id as i64)
            .fetch_one(pool)
            .await?;

        let task = sqlx::query_as!(
            Task,
            "select * from tasks where chat_id = $1 order by description asc offset $2",
            message.chat.id,
            num - 1
        )
        .fetch_optional(pool)
        .await?;

        if let Some(mut task) = task {
            task.done_by = if task.done_by.is_some() { None } else { Some(user.id) };
            sqlx::query!(r#"update tasks set done_by = $1 where id = $2"#, task.done_by, task.id)
                .execute(pool)
                .await?;

            let text = format!(
                "Set task to {}:\n{}",
                if task.done_by.is_some() { "done" } else { "not done" },
                task.description
            );

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
    } else {
        let send_message_params = SendMessageParamsBuilder::default()
            .chat_id(message.chat.id)
            .text("Unknown user.")
            .build()?;

        Ok(send_message_params)
    }
}

async fn get_task_list_string(message: &Message, pool: &Pool<Postgres>) -> Result<String, LeditError> {
    let tasks = sqlx::query_as!(
        Task,
        "select * from tasks where chat_id = $1 order by description asc",
        message.chat.id,
    )
    .fetch_all(pool)
    .await?;

    let mut text = "List of all tasks:\n".to_string();
    let mut n = 1;
    for task in &tasks {
        let checkbox = if task.done_by.is_some() { "✅" } else { "☑️" };
        let recurring = if let Some(interval_days) = task.interval_days {
            format!(
                "(every {} day{})",
                interval_days,
                if interval_days > 1 { "s" } else { "" }
            )
        } else {
            "".to_string()
        };
        text.push_str(&format!("\n {}. {} {} {}", n, checkbox, task.description, recurring));
        n += 1;
    }

    if tasks.is_empty() {
        text = "No task found.".to_string();
    }

    Ok(text)
}

pub async fn get_todos(chat_id: i64, pool: &Pool<Postgres>) -> Result<String, LeditError> {
    // get all tasks that are scheduled for today or earlier
    let tasks_by_username = sqlx::query!(
        r#"
            select 
                t.id,
                t.chat_id,
                t.description,
                t.interval_days,
                t.assigned_user,
                t.scheduled_for,
                t.done_by,

                c.username
            from tasks as t
            join chat_members as c on c.id = t.assigned_user
            where 
                t.chat_id = $1 and t.scheduled_for <= $2
            order by t.done_by desc, t.description asc
        "#,
        chat_id,
        today(),
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .into_group_map_by(|v| v.username.clone());

    // compose response message
    let text = if tasks_by_username.is_empty() {
        "No todos for today :)".to_string()
    } else {
        tasks_by_username
            .into_iter()
            .map(|(username, tasks)| {
                let mut r: String = format!("Todos for {}:\n", username);
                if tasks.is_empty() {
                    r.push_str("\nNo todos for today :)")
                } else {
                    for task in tasks {
                        let checkbox = if let Some(done_by) = task.done_by {
                            if done_by == task.assigned_user {
                                "✅"
                            } else {
                                "✅↪️"
                            }
                        } else {
                            "☑️"
                        };
                        let delay = if task.scheduled_for < today() { "⏳" } else { "" };

                        r.push_str(&format!("\n{}{} {}", checkbox, delay, task.description));
                    }
                }
                r
            })
            .join("\n\n")
    };

    Ok(text)
}
