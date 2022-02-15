# ledit

![CI](https://github.com/jflessau/ledit/actions/workflows/ci.yml/badge.svg)

Telegram bot for fair task assignment.

("ledit" = **le**t's **d**o **i**t **t**ogether)

## Features

Add todos to todo-lists within telegram chats. Each todo is assigned to a random chat member.

Your roommates adds a tedious todo? He may end up assigned to it himself. But the same thing could happen to you.
This makes assignments fair.

You can also create recurring todos, like cleaning all mirrors every 20 days. Once done, recurring todos will be re-assigned to a random chat member.

Overdue todos are marked with a ⏳-emoji (see the screenshots below).

### Commands

| Command                              | Description                  |
| ------------------------------------ | ---------------------------- |
| `/add Clean kitchen`                 | Add a todo                   |
| `/add every 20 days: Clean mirrors ` | Add a recurring todo         |
| `/todos`                             | Get a numbered list of todos |
| `/check 1`                           | Mark todo #1 as done         |
| `/delete 2`                          | Delete todo #2               |

### Screenshots

- [Add and list todos](screenshots/add-and-list.jpg)
- [Check todo](screenshots/check.jpg)
- [Delete todo](screenshots/delete.jpg)
- [Overdue todo](screenshots/delayed-todo.jpg)

## Development

1. Rename `.env-example` to `.env` and fill it with your credentials.
2. Use `docker-compose up` to spin up the postgres database.
3. Install [sqlx-cli](https://crates.io/crates/sqlx-cli) and run the migrations with `sqlx migrate run`

Happy hacking 😊

## Build & run docker image

Build the docker image with `docker build -t ledit .` and run it with these env vars:

| Env var        | Example value                               | Optional |
| -------------- | ------------------------------------------- | -------- |
| `DATABASE_URL` | `postgres://dbuser:password@localhost:5432` | no       |
| `TOKEN`        | `muchsecretwow123456789`                    | no       |
