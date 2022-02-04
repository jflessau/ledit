use dotenv::dotenv;
use frankenstein::{Api, GetUpdatesParamsBuilder, TelegramApi};
use pretty_env_logger;
use std::env;
#[macro_use]
extern crate log;
mod error;
use sqlx::postgres::PgPoolOptions;

use std::error::Error;

mod action;
mod handler;
use action::Action;
use handler::chat_member::register_chat_member;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    pretty_env_logger::init();

    let token = env::var("TOKEN").expect("missing TOKEN env var");
    let api = Api::new(&token);

    let database_url = env::var("DATABASE_URL").expect("missing DATABASE_URL env var");

    let pool = PgPoolOptions::new().max_connections(16).connect(&database_url).await?;

    let mut update_params_builder = GetUpdatesParamsBuilder::default();
    update_params_builder.allowed_updates(vec!["message".to_string(), "callback_query".to_string()]);

    let mut update_params = update_params_builder.build().unwrap();

    loop {
        let result = api.get_updates(&update_params);

        // println!("result: {:?}", result);

        match result {
            Ok(response) =>
                for update in response.result {
                    // println!("update: {:#?}", update);

                    let response = if let Some(message) = update.message {
                        let action = Action::from_message(&message);
                        println!("register chat member");
                        register_chat_member(&message, &pool).await?;

                        Some(action.execute(&pool).await)
                    } else if let Some(_callback) = update.callback_query {
                        // let action = Action::from_callback(&callback);

                        // Some(action.execute(&pool).await)

                        None
                    } else {
                        None
                    };

                    match response {
                        Some(Err(err)) => {
                            error!("{:#?}", err);
                        },
                        Some(Ok(response)) =>
                            if let Err(err) = api.send_message(&response) {
                                error!("failed to send message: {:?}", err);
                            },
                        _ => {},
                    }

                    update_params = update_params_builder.offset(update.update_id + 1).build().unwrap();
                },
            Err(error) => {
                error!("failed to get updates: {:?}", error);
            },
        }
    }
}
