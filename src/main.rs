use dotenv::dotenv;
use frankenstein::{Api, GetUpdatesParamsBuilder, TelegramApi};
use std::env;
#[macro_use]
extern crate log;
mod error;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use std::error::Error;

mod action;
mod handler;
use action::Action;
use handler::chat_member::register_chat_member;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    pretty_env_logger::init();

    let listen_for_updates_thread = tokio::task::spawn(listen_for_updates());
    let re_schedule_tasks_thread = tokio::task::spawn(re_schedule_tasks());

    let (_, _) = (listen_for_updates_thread.await?, re_schedule_tasks_thread.await?);

    Ok(())
}

async fn listen_for_updates() -> Result<(), error::LeditError> {
    let (pool, api) = get_pool_and_api().await;

    sqlx::migrate!("./migrations").run(&pool).await?;

    println!("listen for updates");
    let mut update_params_builder = GetUpdatesParamsBuilder::default();
    update_params_builder.allowed_updates(vec!["message".to_string(), "callback_query".to_string()]);

    let mut update_params = update_params_builder.build().unwrap();

    loop {
        let result = api.get_updates(&update_params);

        match result {
            Ok(response) =>
                for update in response.result {
                    let response = if let Some(message) = update.message {
                        let action = Action::from_message(&message);
                        register_chat_member(&message, &pool).await?;

                        Some(action.execute(&pool).await)
                    } else if let Some(_callback) = update.callback_query {
                        None
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
                error!("failed to get updates: {:?}", error);
            },
        }
    }
}

async fn re_schedule_tasks() -> Result<(), error::LeditError> {
    let (pool, _) = get_pool_and_api().await;
    loop {
        sleep(Duration::from_millis(100000)).await;

        // re-schedule tasks
        let ids_of_tasks_to_re_schedule = sqlx::query!(
            r#"
                select id
                from tasks
                where 
                    interval_days is not null
                    and done_by is not null
                    and scheduled_for < now() - interval '1 days' * interval_days
            "#
        )
        .fetch_all(&pool)
        .await?
        .into_iter()
        .map(|v| v.id)
        .collect::<Vec<Uuid>>();

        if !ids_of_tasks_to_re_schedule.is_empty() {
            println!("reschedule: {:#?}", ids_of_tasks_to_re_schedule);
        }

        sqlx::query!(
            r#"
                update tasks
                set
                    done_by = null,
                    scheduled_for = now()
                where id = any($1)
            "#,
            &ids_of_tasks_to_re_schedule
        )
        .execute(&pool)
        .await?;

        // delete one-time tasks that are done
        sqlx::query!(
            r#"
                delete from 
                    tasks 
                where 
                    done_by is not null 
                    and interval_days is null 
                    and scheduled_for < now() - interval '1 day'
            "#
        )
        .execute(&pool)
        .await?;
    }
}

async fn get_pool_and_api() -> (Pool<Postgres>, Api) {
    let token = env::var("TOKEN").expect("missing TOKEN env var");
    let api = Api::new(&token);
    let database_url = env::var("DATABASE_URL").expect("missing DATABASE_URL env var");
    let pool = PgPoolOptions::new()
        .max_connections(16)
        .connect(&database_url)
        .await
        .expect("failed to get connection pool");

    (pool, api)
}
