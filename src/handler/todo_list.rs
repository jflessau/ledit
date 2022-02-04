use crate::{
    error::LeditError,
    handler::{chat_member::ChatMember, task::Task},
};
use chrono::{DateTime, Utc};
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
    pub completed_at: Option<DateTime<Utc>>,
    pub assigned_user: Option<Uuid>,
    pub completed_by: Option<Uuid>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TodoListItem {
    chat_member_id: Uuid,
    username: String,
    task_id: Uuid,
    description: String,
}

pub async fn handle_get_todo_lists(chat_id: i64, pool: &Pool<Postgres>) -> Result<SendMessageParams, LeditError> {
    println!("a");

    generate_todo_lists(chat_id, pool).await?;
    println!("b");

    let todo_list_items = get_todo_list_items(chat_id, pool).await?;

    println!("c");
    let todo_lists = todo_list_items
        .into_iter()
        .map(|v| (v.username.clone(), v))
        .into_group_map()
        .into_iter()
        .map(|(username, todo_list_items)| {
            let tasks_str = todo_list_items
                .into_iter()
                .map(|v| (v.description.clone(), v))
                .into_group_map()
                .into_iter()
                .map(|(description, tasks)| format!("\n{}x {}", tasks.len(), description))
                .collect::<Vec<String>>()
                .join("");
            format!("☑️ {}{}", username, tasks_str)
        })
        .collect::<Vec<String>>()
        .join("\n\n");

    println!("todo_lists: {:#?}", todo_lists);

    let send_message_params = SendMessageParamsBuilder::default()
        .chat_id(chat_id)
        .text(todo_lists)
        .build()?;

    Ok(send_message_params)
}

pub async fn get_todo_list_items(chat_id: i64, pool: &Pool<Postgres>) -> Result<Vec<TodoListItem>, LeditError> {
    let r = sqlx::query_as!(
        TodoListItem,
        r#" 
            select
                cm.id as chat_member_id, 
                cm.username,
                t.id as task_id,
                t.description
            from executions as e
            join tasks as t on t.id = e.task_id
            join chat_members as cm on cm.id = e.assigned_user
            where t.chat_id = $1 and completed_at is null
         "#,
        chat_id,
    )
    .fetch_all(pool)
    .await?;

    Ok(r)
}

pub async fn generate_todo_lists(chat_id: i64, pool: &Pool<Postgres>) -> Result<(), LeditError> {
    println!("generate_todo_lists");

    // TODO - get all task that are not recurring and have no execution
    //        or recurring tasks without an recent execution

    let chat_member_ids = sqlx::query_as!(
        ChatMember,
        r#"
            select * from chat_members where chat_id = $1
        "#,
        chat_id
    )
    .fetch_all(pool)
    .await?
    .iter()
    .map(|v| v.id)
    .collect::<Vec<Uuid>>();

    println!("chat_member_ids: {:#?}", chat_member_ids);

    let new_executions = sqlx::query_as!(
        Task,
        r#"
            select t.* from tasks as t
            left outer join executions as e on e.task_id = t.id
            where chat_id = $1 and e.id is null
        "#,
        chat_id,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|v| Execution {
        id: Uuid::new_v4(),
        task_id: v.id,
        completed_at: None,
        assigned_user: chat_member_ids.choose(&mut rand::thread_rng()).cloned(),
        completed_by: None,
    })
    .collect::<Vec<Execution>>();

    println!("new_executions: {:#?}", new_executions);

    for execution in new_executions {
        sqlx::query_as!(
            Execution,
            r#"
                insert into executions (
                    id,
                    task_id,
                    completed_at,
                    assigned_user,
                    completed_by
                )
                values ( $1, $2, $3, $4, $5 )
            "#,
            execution.id,
            execution.task_id,
            execution.completed_at,
            execution.assigned_user,
            execution.completed_by,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}
