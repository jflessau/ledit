#[macro_use]
extern crate log;
use dotenv::dotenv;
use frankenstein::{GetUpdatesParamsBuilder, TelegramApi};
use std::error::Error;

mod action;
mod error;
mod handler;
mod interval;
mod util;
use action::Action;
use handler::chat_member::register_chat_member;
use interval::interval_actions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // setup logging
    dotenv().ok();
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info,sqlx=warn")
    }
    tracing_subscriber::fmt::init();

    // starting two threads, one for processign telegram api updates and one for time based db interactions
    tracing::info!("starting bot...");
    let listen_for_updates_thread = tokio::task::spawn(listen_for_updates());
    let re_schedule_todos_thread = tokio::task::spawn(interval_actions());
    let (_, _) = (listen_for_updates_thread.await?, re_schedule_todos_thread.await?);

    Ok(())
}

async fn listen_for_updates() -> Result<(), error::LeditError> {
    let (pool, api) = util::get_pool_and_api().await;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let mut update_params_builder = GetUpdatesParamsBuilder::default();
    update_params_builder.allowed_updates(vec!["message".to_string(), "callback_query".to_string()]);

    let mut update_params = update_params_builder.build().unwrap();

    loop {
        let result = api.get_updates(&update_params);
        tracing::debug!("received telegram api update");

        match result {
            Ok(response) =>
                for update in response.result {
                    let response = if let Some(message) = update.message {
                        let action = Action::from_message(&message);
                        tracing::info!("action: {}", action);

                        if let Err(err) = register_chat_member(&message, &pool).await {
                            tracing::error!("failed to register chat member, err: {}", err);
                        }

                        let response = action.execute(&pool).await;
                        match response {
                            Ok(_) => Some(response),
                            Err(err) => {
                                tracing::error!("failed to respond to action, err: {}", err);
                                None
                            },
                        }
                    } else {
                        None
                    };

                    match response {
                        Some(Err(err)) => {
                            error!("{:#?}", err);
                        },
                        Some(Ok(Some(response))) =>
                            if let Err(err) = api.send_message(&response) {
                                error!("failed to send message: {:?}", err);
                            },
                        _ => {},
                    }

                    update_params = update_params_builder.offset(update.update_id + 1).build().unwrap();
                },
            Err(error) => {
                tracing::error!("failed to process telegram api update, err: {:?}", error);
            },
        }
    }
}
