create extension if not exists "uuid-ossp";

create table chat_members (
  id uuid primary key not null,
  telegram_user_id int8 not null,
  chat_id int8 not null,
  username text not null
);

create table tasks (
  id uuid primary key not null,
  chat_id int8 not null,
  description text not null,
  interval_days int8
);

create table executions (
  id uuid not null primary key,
  task_id uuid not null,
  completed_at timestamptz,
  assigned_user uuid not null,
  completed_by int8
);

create table ratings (
  id uuid not null,
  execution_id uuid not null,
  thumbs_up boolean
);
