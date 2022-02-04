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
  description text not null,
  interval_days int8
);

create table executions (
  id uuid not null primary key,
  task_id uuid not null,
  assigned_user uuid not null,
  completed_at timestamptz,
  completed_by uuid
);
