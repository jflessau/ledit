use crate::error::LeditError;
use frankenstein::{Message, SendMessageParams, SendMessageParamsBuilder};
use std::fs::read_to_string;

pub fn handle_help(message: &Message) -> Result<SendMessageParams, LeditError> {
    let help_text = read_to_string("./txt/help.txt").unwrap();

    let send_message_params = SendMessageParamsBuilder::default()
        .chat_id(message.chat.id)
        .text(&help_text)
        .build()?;

    Ok(send_message_params)
}

pub fn handle_start(message: &Message) -> Result<SendMessageParams, LeditError> {
    let help_text = read_to_string("./txt/start.txt").unwrap();

    let send_message_params = SendMessageParamsBuilder::default()
        .chat_id(message.chat.id)
        .text(&help_text)
        .build()?;

    Ok(send_message_params)
}
