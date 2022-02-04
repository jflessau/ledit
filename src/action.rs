use crate::{
    error::LeditError,
    handler::{
        info::{handle_help, handle_start},
        task::{handle_add_task, handle_delete_task, handle_list_tasks, handle_set_task_repetition},
        todos::handle_get_todos,
    },
};
use frankenstein::{objects::CallbackQuery, Message, SendMessageParams, SendMessageParamsBuilder};
use regex::Regex;
use sqlx::{Pool, Postgres};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug)]
pub enum Action<'a> {
    UnknownMessage(&'a Message),
    UnknownCallback(&'a CallbackQuery),
    Start(&'a Message),
    Help(&'a Message),
    AddTask { title: String, message: &'a Message },
    ListTasks(&'a Message),
    DeleteTask { num: i64, message: &'a Message },
    GetTodos { chat_id: i64 },
}

impl<'a> Action<'a> {
    pub fn from_message(message: &'a Message) -> Self {
        let s = message.text.clone().unwrap_or_else(|| "".to_string());

        // start
        let start_re = Regex::new(r"\A((?i)/start(?-i))").expect("start_re construction failed");
        if start_re.captures(&s).is_some() {
            return Action::Start(message);
        }

        // help
        let help_re = Regex::new(r"\A((?i)/help(?-i))").expect("help_re construction failed");
        if help_re.captures(&s).is_some() {
            return Action::Help(message);
        }

        // add task
        let add_task_re =
            Regex::new(r"\A((?i)/add(?-i)([ ]+)([a-zA-Z0-9\-_:,. ].{0,42}))").expect("add_task_re construction failed");
        if let Some(caps) = add_task_re.captures(&s) {
            let title: String = caps.get(3).expect("add_re caps failed").as_str().to_string();
            return Action::AddTask { title, message };
        }

        // list tasks
        let list_tasks_re = Regex::new(r"\A((?i)/list(?-i))").expect("list_re construction failed");
        if list_tasks_re.captures(&s).is_some() {
            return Action::ListTasks(message);
        }

        // delete task
        let list_task_re = Regex::new(r"((?i)/delete(?-i))[ ]+([0-9]+)").expect("building list_task_re failed");
        if let Some(caps) = list_task_re.captures(&s) {
            let num = caps.get(2).expect("caps get 2 failed").as_str().parse().unwrap_or(1);
            return Action::DeleteTask { num, message };
        }

        // get todos
        let get_todo_re = Regex::new(r"\A((?i)/todos(?-i))").expect("get_todos_re construction failed");
        if get_todo_re.captures(&s).is_some() {
            return Action::GetTodos {
                chat_id: message.chat.id,
            };
        }

        // unknown
        Action::UnknownMessage(message)
    }

    // pub fn from_callback(callback: &'a CallbackQuery) -> Self {
    //     let s = callback.data.clone().unwrap_or_else(|| "".to_string());

    //     // set task repetition
    //     let task_recurring_re = Regex::new(r"\A(?i)/callback(?-i)([ ]*)([a-f0-9-]*)([ ]*)([0-9]{1,3})")
    //         .expect("building task_recurring_re failed");
    //     if let Some(caps) = task_recurring_re.captures(&s) {
    //         let uuid_str = caps
    //             .get(2)
    //             .expect("caps get 2 failed")
    //             .as_str()
    //             .parse()
    //             .unwrap_or(Uuid::new_v4().to_string());
    //         let interval_days: i64 = caps.get(4).expect("caps get 4 failed").as_str().parse().unwrap_or(1);
    //         if let (Ok(id), Some(message)) = (Uuid::from_str(&uuid_str), callback.message.as_ref()) {
    //             return Action::SetTaskInterval {
    //                 task_id: id,
    //                 interval_days,
    //                 chat_id: message.chat.id,
    //             };
    //         }
    //     }

    //     // unknown
    //     Action::UnknownCallback(callback)
    // }

    pub async fn execute(self, pool: &Pool<Postgres>) -> Result<SendMessageParams, LeditError> {
        let res = match self {
            Action::Help(message) => handle_help(message)?,
            Action::Start(message) => handle_start(message)?,
            Action::AddTask { title, message } => handle_add_task(title, message, pool).await?,
            Action::ListTasks(message) => handle_list_tasks(message, pool).await?,
            Action::DeleteTask { num, message } => handle_delete_task(num, message, pool).await?,
            Action::GetTodos { chat_id } => handle_get_todos(chat_id, pool).await?,
            Action::UnknownMessage(message) => SendMessageParamsBuilder::default()
                .chat_id(message.chat.id)
                .text("Say what?")
                .build()?,
            Action::UnknownCallback(callback) => SendMessageParamsBuilder::default()
                .chat_id(callback.message.as_ref().unwrap().chat.id)
                .text("Say what?")
                .build()?,
        };

        Ok(res)
    }
}
