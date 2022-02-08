use crate::{
    error::LeditError,
    handler::{
        info::{handle_help, handle_start},
        task::{handle_add_task, handle_check_task, handle_delete_task, handle_list_todos},
    },
};
use frankenstein::{Message, SendMessageParams};
use regex::Regex;
use sqlx::{Pool, Postgres};
use std::fmt;

#[derive(Debug)]
pub enum Action<'a> {
    UnknownMessage,
    Start(&'a Message),
    Help(&'a Message),
    AddTask {
        title: String,
        interval_days: Option<i64>,
        message: &'a Message,
    },
    ListTodos(&'a Message),
    DeleteTask {
        num: i64,
        message: &'a Message,
    },
    CheckTask {
        num: i64,
        message: &'a Message,
    },
}

impl fmt::Display for Action<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = match self {
            Action::UnknownMessage => "UnknownMessage".to_string(),
            Action::Start(_) => "Start".to_string(),
            Action::Help(_) => "Help".to_string(),
            Action::AddTask {
                title, interval_days, ..
            } => format!("AddTask {{ title: {}, interval_days: {:?} }}", title, interval_days),
            Action::ListTodos(_) => "ListTodos".to_string(),
            Action::DeleteTask { num, .. } => format!("DeleteTask {{ num: {} }}", num),
            Action::CheckTask { num, .. } => format!("CheckTask: {{ num: {} }}", num),
        };

        write!(f, "{}", text)
    }
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
            return Action::ListTodos(message);
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
        tracing::info!("received unknown action, text: {}", s);
        Action::UnknownMessage
    }

    pub async fn execute(self, pool: &Pool<Postgres>) -> Result<Option<SendMessageParams>, LeditError> {
        let res = match self {
            Action::Help(message) => Some(handle_help(message)?),
            Action::Start(message) => Some(handle_start(message)?),
            Action::AddTask {
                title,
                interval_days,
                message,
            } => Some(handle_add_task(title, interval_days, message, pool).await?),
            Action::ListTodos(message) => Some(handle_list_todos(message, pool).await?),
            Action::DeleteTask { num, message } => Some(handle_delete_task(num, message, pool).await?),
            Action::CheckTask { num, message } => Some(handle_check_task(num, message, pool).await?),
            Action::UnknownMessage => None,
        };

        Ok(res)
    }
}
