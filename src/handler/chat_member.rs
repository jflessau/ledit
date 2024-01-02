use crate::error::LeditError;
use frankenstein::{objects::User, Message};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Postgres};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct ChatMember {
    pub id: Uuid,
    pub telegram_user_id: i64,
    pub chat_id: i64,
    pub username: String,
    pub todo_weight: i64,
}

pub async fn register_chat_member(
    message: &Message,
    pool: &Pool<Postgres>,
) -> Result<(), LeditError> {
    if let Some(User {
        id,
        first_name,
        username,
        ..
    }) = message.from.as_ref()
    {
        let chat_member = sqlx::query_as!(
            ChatMember,
            "select * from chat_members where telegram_user_id = $1 and chat_id = $2",
            *id as i64,
            message.chat.id,
        )
        .fetch_optional(pool)
        .await?;

        if chat_member.is_none() {
            info!("register new chat member");

            sqlx::query_as!(
                ChatMember,
                r#"
                    insert into chat_members (
                        id,
                        telegram_user_id,
                        chat_id,
                        username
                    )
                    values ( $1, $2, $3, $4 )
                "#,
                Uuid::new_v4(),
                *id as i64,
                message.chat.id,
                username.clone().unwrap_or_else(|| first_name.to_string()),
            )
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

pub async fn get_random_chat_member(
    chat_id: i64,
    pool: &Pool<Postgres>,
) -> Result<Uuid, LeditError> {
    let users = sqlx::query!(r#"select id from chat_members where chat_id = $1"#, chat_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|v| v.id)
        .collect::<Vec<Uuid>>();

    let mut rng: StdRng = SeedableRng::from_entropy();
    let n = rng.gen_range(0..users.len());

    match users.get(n).cloned() {
        Some(v) => Ok(v),
        None => Err(LeditError::RndUser),
    }
}
