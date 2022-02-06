use crate::{
    error::LeditError,
    handler::{
        info::{handle_help, handle_start},
        task::{handle_add_task, handle_check_task, handle_delete_task, handle_list_tasks},
    },
};
use frankenstein::{Message, SendMessageParams};
use regex::Regex;
use sqlx::{Pool, Postgres};

#[derive(Debug)]
pub enum Action<'a> {
    UnknownMessage,
    // UnknownCallback(&'a CallbackQuery),
    Start(&'a Message),
    Help(&'a Message),
    AddTask {
        title: String,
        interval_days: Option<i64>,
        message: &'a Message,
    },
    ListTasks(&'a Message),
    DeleteTask {
        num: i64,
        message: &'a Message,
    },
    CheckTask {
        num: i64,
        message: &'a Message,
    },
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

        // add recurring task
        let add_task_re = Regex::new(
            r"\A((?i)/add(?-i))[ ]+((?i)every(?i))[ ]+([0-9]+)[ ]+((?i)day(?i))(s){0,1}:[ ]+([a-zA-Z0-9\-_:,. ].{0,42})",
        )
        .expect("add_recurring_task_re construction failed");
        if let Some(caps) = add_task_re.captures(&s) {
            let interval_str: String = caps
                .get(3)
                .expect("recurring_task_interval_re caps failed")
                .as_str()
                .to_string();
            let title: String = caps
                .get(6)
                .expect("recurring_task_title_re caps failed")
                .as_str()
                .to_string();
            return Action::AddTask {
                title,
                interval_days: interval_str.parse::<i64>().ok(),
                message,
            };
        }

        // add one-time task
        let add_task_re = Regex::new(r"\A((?i)/add(?-i)([ ]+)([a-zA-Z0-9\-_:,. ].{0,42}))")
            .expect("add_one_time_task_re construction failed");
        if let Some(caps) = add_task_re.captures(&s) {
            let title: String = caps.get(3).expect("add_re caps failed").as_str().to_string();
            return Action::AddTask {
                title,
                interval_days: None,
                message,
            };
        }

        // list todos
        let list_tasks_re = Regex::new(r"\A((?i)/todos(?-i))").expect("list_todos_re construction failed");
        if list_tasks_re.captures(&s).is_some() {
            return Action::ListTasks(message);
        }

        // delete task
        let delete_task_re = Regex::new(r"((?i)/delete(?-i))[ ]+([0-9]+)").expect("building delete_task_re failed");
        if let Some(caps) = delete_task_re.captures(&s) {
            let num = caps.get(2).expect("caps get 2 failed").as_str().parse().unwrap_or(1);
            return Action::DeleteTask { num, message };
        }

        // check task
        let check_task_re = Regex::new(r"((?i)/check(?-i))[ ]+([0-9]+)").expect("building check_task_re failed");
        if let Some(caps) = check_task_re.captures(&s) {
            let num = caps.get(2).expect("caps get 2 failed").as_str().parse().unwrap_or(1);
            return Action::CheckTask { num, message };
        }

        // unknown
        Action::UnknownMessage
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

    pub async fn execute(self, pool: &Pool<Postgres>) -> Result<Option<SendMessageParams>, LeditError> {
        let res = match self {
            Action::Help(message) => Some(handle_help(message)?),
            Action::Start(message) => Some(handle_start(message)?),
            Action::AddTask {
                title,
                interval_days,
                message,
            } => Some(handle_add_task(title, interval_days, message, pool).await?),
            Action::ListTasks(message) => Some(handle_list_tasks(message, pool).await?),
            Action::DeleteTask { num, message } => Some(handle_delete_task(num, message, pool).await?),
            Action::CheckTask { num, message } => Some(handle_check_task(num, message, pool).await?),
            Action::UnknownMessage => None
            // Action::UnknownCallback(callback) => SendMessageParamsBuilder::default()
            //     .chat_id(callback.message.as_ref().unwrap().chat.id)
            //     .text("Say what?")
            //     .build()?,
        };

        Ok(res)
    }
}
