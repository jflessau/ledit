use crate::{
    error::LeditError,
    handler::{
        info::{handle_help, handle_start},
        todo::{handle_add_todo, handle_check_todo, handle_delete_todo, handle_list_todos},
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
    AddTodo {
        title: String,
        interval_days: Option<i64>,
        message: &'a Message,
    },
    ListTodos(&'a Message),
    DeleteTodo {
        num: i64,
        message: &'a Message,
    },
    CheckTodo {
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
            Action::AddTodo {
                title, interval_days, ..
            } => format!("AddTodo {{ title: {}, interval_days: {:?} }}", title, interval_days),
            Action::ListTodos(_) => "ListTodos".to_string(),
            Action::DeleteTodo { num, .. } => format!("DeleteTodo {{ num: {} }}", num),
            Action::CheckTodo { num, .. } => format!("CheckTodo: {{ num: {} }}", num),
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

        // add recurring todo
        let add_todo_re = Regex::new(
            r"\A((?i)/add(?-i))[ ]+((?i)every(?i))[ ]+([0-9]+)[ ]+((?i)day(?i))(s){0,1}:[ ]+([a-zA-Z0-9\-_:,. ].{0,64})",
        )
        .expect("add_recurring_todo_re construction failed");
        if let Some(caps) = add_todo_re.captures(&s) {
            let interval_str: String = caps
                .get(3)
                .expect("recurring_todo_interval_re caps failed")
                .as_str()
                .to_string();
            let title: String = caps
                .get(6)
                .expect("recurring_todo_title_re caps failed")
                .as_str()
                .to_string();
            return Action::AddTodo {
                title,
                interval_days: interval_str.parse::<i64>().ok(),
                message,
            };
        }

        // add one-time todo
        let add_todo_re = Regex::new(r"\A((?i)/add(?-i)([ ]+)([a-zA-Z0-9\-_:,. ].{0,64}))")
            .expect("add_one_time_todo_re construction failed");
        if let Some(caps) = add_todo_re.captures(&s) {
            let title: String = caps.get(3).expect("add_re caps failed").as_str().to_string();
            return Action::AddTodo {
                title,
                interval_days: None,
                message,
            };
        }

        // list todos
        let list_todos_re = Regex::new(r"\A((?i)/todos(?-i))").expect("list_todos_re construction failed");
        if list_todos_re.captures(&s).is_some() {
            return Action::ListTodos(message);
        }

        // delete todo
        let delete_todo_re = Regex::new(r"((?i)/delete(?-i))[ ]+([0-9]+)").expect("building delete_todo_re failed");
        if let Some(caps) = delete_todo_re.captures(&s) {
            let num = caps.get(2).expect("caps get 2 failed").as_str().parse().unwrap_or(1);
            return Action::DeleteTodo { num, message };
        }

        // check todo
        let check_todo_re = Regex::new(r"((?i)/check(?-i))[ ]+([0-9]+)").expect("building check_todo_re failed");
        if let Some(caps) = check_todo_re.captures(&s) {
            let num = caps.get(2).expect("caps get 2 failed").as_str().parse().unwrap_or(1);
            return Action::CheckTodo { num, message };
        }

        // unknown
        tracing::info!("received unknown action, text: {}", s);
        Action::UnknownMessage
    }

    pub async fn execute(self, pool: &Pool<Postgres>) -> Result<Option<SendMessageParams>, LeditError> {
        let res = match self {
            Action::Help(message) => Some(handle_help(message)?),
            Action::Start(message) => Some(handle_start(message)?),
            Action::AddTodo {
                title,
                interval_days,
                message,
            } => Some(handle_add_todo(title, interval_days, message, pool).await?),
            Action::ListTodos(message) => Some(handle_list_todos(message, pool).await?),
            Action::DeleteTodo { num, message } => Some(handle_delete_todo(num, message, pool).await?),
            Action::CheckTodo { num, message } => Some(handle_check_todo(num, message, pool).await?),
            Action::UnknownMessage => None,
        };

        Ok(res)
    }
}
