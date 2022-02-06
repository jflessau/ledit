create extension if not exists "uuid-ossp";

create table chat_members (
  id uuid primary key not null,
  telegram_user_id int8 not null,
  chat_id int8 not null,
  username text not null,
  task_weight int8 not null default 100
);

create table tasks (
  id uuid primary key not null,
  chat_id int8 not null,
  "description" text not null,
  --
  interval_days int8,
  assigned_user uuid not null,
  scheduled_for date not null default now(),
  done_by uuid
);
