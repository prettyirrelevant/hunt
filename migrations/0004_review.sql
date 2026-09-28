create table drafts (
    job_id        bigint primary key references jobs (id) on delete cascade,
    cv            jsonb not null,
    cv_changes    jsonb not null default '[]',
    letter        text not null,
    email_to      text,
    email_subject text,
    answers       jsonb not null default '[]',
    provider      text not null,
    created_at    timestamptz not null default now()
);
