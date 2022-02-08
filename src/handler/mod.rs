use chrono::{Datelike, NaiveDate, Utc};
use frankenstein::{
    api_params::ReplyMarkup,
    objects::{InlineKeyboardButton, InlineKeyboardMarkup},
};

pub mod chat_member;
pub mod info;
pub mod todo;

pub fn _simple_inline_keyboard(button_data: Vec<(String, String)>) -> ReplyMarkup {
    let buttons = button_data
        .into_iter()
        .map(|(label, callback_str)| {
            vec![InlineKeyboardButton {
                text: label,
                url: None,
                login_url: None,
                callback_data: Some(callback_str),
                switch_inline_query: None,
                switch_inline_query_current_chat: None,
                callback_game: None,
                pay: None,
            }]
        })
        .collect::<Vec<Vec<InlineKeyboardButton>>>();

    ReplyMarkup::InlineKeyboardMarkup(InlineKeyboardMarkup {
        inline_keyboard: buttons,
    })
}

pub fn today() -> NaiveDate {
    let today = Utc::today();
    NaiveDate::from_ymd(today.year(), today.month(), today.day())
}
