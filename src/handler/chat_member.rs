use crate::error::LeditError;
use frankenstein::{objects::User, Message};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Postgres};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct ChatMember {
    pub id: Uuid,
    pub telegram_user_id: i64,
    pub chat_id: i64,
    pub username: String,
}

pub async fn register_chat_member(message: &Message, pool: &Pool<Postgres>) -> Result<(), LeditError> {
    if let Some(User {
        id,
        first_name,
        username,
        ..
    }) = message.from.as_ref()
    {
        let chat_member = sqlx::query_as!(
            ChatMember,
            "select * from chat_members where telegram_user_id = $1 and chat_id=$2",
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
