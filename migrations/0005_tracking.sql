create table messages (
    id         bigint generated always as identity primary key,
    uid        bigint not null unique,
    message_id text not null,
    at         timestamptz not null,
    sender     text not null,
    subject    text not null,
    snippet    text not null,
    job_id     bigint references jobs (id) on delete set null,
    class      text,
    confidence real,
    status     text not null default 'new'  -- applied | needs_you | ignored
);
