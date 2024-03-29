use crate::{error::LeditError, handler::chat_member::get_random_chat_member, util::today};
use chrono::NaiveDate;
use frankenstein::{objects::User, Message, SendMessageParams, SendMessageParamsBuilder};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Postgres};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct Todo {
    pub id: Uuid,
    pub chat_id: i64,
    pub description: String,

    pub interval_days: Option<i64>,
    pub assigned_user: Uuid,
    pub scheduled_for: NaiveDate,
    pub done_by: Option<Uuid>,
}
pub async fn handle_add_todo(
    title: String,
    mut interval_days: Option<usize>,
    message: &Message,
    pool: &Pool<Postgres>,
) -> Result<SendMessageParams, LeditError> {
    if let Some(v) = interval_days {
        if !(1..=999).contains(&v) {
            interval_days = None
        }
    };

    let assigned_user = get_random_chat_member(message.chat.id, pool).await?;

    let todo = sqlx::query_as!(
        Todo,
        r#"
            insert into todos (
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
        interval_days.map(|v| v as i64),
        assigned_user,
    )
    .fetch_one(pool)
    .await?;

    let send_message_params = SendMessageParamsBuilder::default()
        .chat_id(message.chat.id)
        .text(format!("Added: {}", todo.description))
        .build()?;

    Ok(send_message_params)
}

pub async fn handle_list_todos(
    message: &Message,
    pool: &Pool<Postgres>,
) -> Result<SendMessageParams, LeditError> {
    let mut text = get_all_todos_as_string(message, pool).await?;

    text.push_str("\n\n\n");
    text.push_str(&get_todos_by_username_as_string(message.chat.id, pool).await?);

    let send_message_params = SendMessageParamsBuilder::default()
        .chat_id(message.chat.id)
        .text(text)
        .build()?;

    Ok(send_message_params)
}

pub async fn handle_delete_todo(
    num: usize,
    message: &Message,
    pool: &Pool<Postgres>,
) -> Result<SendMessageParams, LeditError> {
    let todos = get_sorted_todos(message.chat.id, pool).await?;
    let todo_to_delete = todos.get(num.saturating_sub(1));

    if let Some(todo_to_delete) = todo_to_delete {
        sqlx::query!(
            r#"
                delete from todos where id = $1
            "#,
            todo_to_delete.id
        )
        .execute(pool)
        .await?;

        let mut text = format!("Deleted: {}\n\n", todo_to_delete.description);
        text.push_str(&get_all_todos_as_string(message, pool).await?);

        let send_message_params = SendMessageParamsBuilder::default()
            .chat_id(message.chat.id)
            .text(text)
            .build()?;

        Ok(send_message_params)
    } else {
        let send_message_params = SendMessageParamsBuilder::default()
            .chat_id(message.chat.id)
            .text("Todo not found.")
            .build()?;

        Ok(send_message_params)
    }
}

pub async fn handle_check_todo(
    num: usize,
    message: &Message,
    pool: &Pool<Postgres>,
) -> Result<SendMessageParams, LeditError> {
    if let Some(User { id, .. }) = message.from.as_ref() {
        let user = sqlx::query!(
            "select id from chat_members where telegram_user_id = $1 and chat_id = $2",
            *id as i64,
            message.chat.id
        )
        .fetch_one(pool)
        .await?;

        let todos = get_sorted_todos(message.chat.id, pool).await?;
        let todo_to_check = todos.get(num.saturating_sub(1)).cloned();

        if let Some(mut todo) = todo_to_check {
            todo.done_by = if todo.done_by.is_some() {
                None
            } else {
                Some(user.id)
            };
            sqlx::query!(
                r#"update todos set done_by = $1 where id = $2"#,
                todo.done_by,
                todo.id
            )
            .execute(pool)
            .await?;

            let text = format!(
                "{} {}",
                if todo.done_by.is_some() {
                    "✅"
                } else {
                    "☑️"
                },
                todo.description
            );

            let send_message_params = SendMessageParamsBuilder::default()
                .chat_id(message.chat.id)
                .text(text)
                .build()?;

            Ok(send_message_params)
        } else {
            let send_message_params = SendMessageParamsBuilder::default()
                .chat_id(message.chat.id)
                .text("Todo not found.")
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

async fn get_sorted_todos(chat_id: i64, pool: &Pool<Postgres>) -> Result<Vec<Todo>, LeditError> {
    sqlx::query_as!(
        Todo,
        r#"
            select 
                * 
            from 
                todos 
            where 
                chat_id = $1 
            order by 
                interval_days is null desc, interval_days asc, description asc
        "#,
        chat_id,
    )
    .fetch_all(pool)
    .await
    .map_err(|err| err.into())
}

async fn get_all_todos_as_string(
    message: &Message,
    pool: &Pool<Postgres>,
) -> Result<String, LeditError> {
    let todos = get_sorted_todos(message.chat.id, pool).await?;

    let mut text = "List of all todos:\n".to_string();
    let mut n = 1;
    for todo in &todos {
        let checkbox = if todo.done_by.is_some() {
            if todo.scheduled_for == today() {
                "✅"
            } else {
                "🗓"
            }
        } else {
            "☑️"
        };
        let recurring = if let Some(interval_days) = todo.interval_days {
            format!(
                "(🔄 {} day{})",
                interval_days,
                if interval_days > 1 { "s" } else { "" }
            )
        } else {
            "".to_string()
        };
        text.push_str(&format!(
            "\n{} {}. {} {} ",
            checkbox, n, todo.description, recurring
        ));
        n += 1;
    }

    if todos.is_empty() {
        text = "No todo found.".to_string();
    }

    Ok(text)
}

pub async fn get_todos_by_username_as_string(
    chat_id: i64,
    pool: &Pool<Postgres>,
) -> Result<String, LeditError> {
    // get actionable todos

    let mut todos_by_username = sqlx::query!(
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
            from 
                todos as t
            join 
                chat_members as c on c.id = t.assigned_user
            where 
                t.chat_id = $1
                and c.chat_id = $1
                and 
                    (
                        (t.interval_days is null and t.scheduled_for <= $2)
                        or 
                        (   
                            t.interval_days is not null
                            and (
                                (t.scheduled_for <= $2 and t.done_by is null) 
                                or (t.scheduled_for = $2 and t.done_by is not null))
                        )
                    )
            order by 
                t.done_by asc, t.description asc
        "#,
        chat_id,
        today(),
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .into_group_map_by(|v| v.username.clone())
    .into_iter()
    .collect::<Vec<(String, _)>>();

    todos_by_username.sort_by(|(a, _), (b, _)| b.partial_cmp(a).unwrap());

    // compose response message

    let text = if todos_by_username.is_empty() {
        "No todos for today :)".to_string()
    } else {
        todos_by_username
            .into_iter()
            .map(|(username, todos)| {
                let mut r: String = format!("Todos for {}:\n", username);
                if todos.is_empty() {
                    r.push_str("\nNo todos for today :)")
                } else {
                    for todo in todos {
                        let checkbox = if let Some(done_by) = todo.done_by {
                            if done_by == todo.assigned_user {
                                "✅"
                            } else {
                                "✅↪️"
                            }
                        } else {
                            "☑️"
                        };
                        let delay = if todo.scheduled_for < today() && todo.done_by.is_none() {
                            "⏳"
                        } else {
                            ""
                        };

                        r.push_str(&format!("\n{}{} {}", checkbox, delay, todo.description));
                    }
                }
                r
            })
            .join("\n\n")
    };

    Ok(text)
}
