create extension if not exists vector;
create extension if not exists pg_trgm;

-- array_to_string is only stable, and generated columns need immutable.
create function words(text[]) returns text
    language sql immutable parallel safe
    return array_to_string($1, ' ');

create table jobs (
    id           bigint generated always as identity primary key,
    key          text not null unique,          -- source:external id
    fingerprint  text not null,                 -- company + title words, for cross-source dedup
    source       text not null,
    url          text not null,
    apply_url    text not null,
    company      text not null,
    title        text not null,
    location     text not null default '',
    work_mode    text not null default 'unknown',
    regions      text[] not null default '{}',
    countries    text[] not null default '{}',
    visa         boolean,
    relocation   boolean,
    salary_min   bigint,
    salary_max   bigint,
    currency     text,
    seniority    text,
    skills       text[] not null default '{}',
    requirements jsonb not null default '[]',
    description  text not null default '',
    posted_at    timestamptz,
    first_seen   timestamptz not null default now(),
    last_seen    timestamptz not null default now(),
    stage        text not null default 'new',
    stage_at     timestamptz not null default now(),
    fit          integer,                       -- AI fit, 0-100
    like_score   real,                          -- learned from your decisions, 0-1
    odds         real,                          -- learned from replies, 0-1
    score        integer,                       -- what the queue sorts by
    assessment   jsonb,
    flags        text[] not null default '{}',
    apply_via    text,                          -- email | form | manual
    embedding    vector(256),
    search       tsvector generated always as (
        setweight(to_tsvector('english', title), 'A') ||
        setweight(to_tsvector('english', company), 'A') ||
        setweight(to_tsvector('english', words(skills)), 'B') ||
        setweight(to_tsvector('english', left(description, 20000)), 'C')
    ) stored
);
create index jobs_stage on jobs (stage, score desc nulls last);
create index jobs_fingerprint on jobs (fingerprint);
create index jobs_search on jobs using gin (search);
create index jobs_company_trgm on jobs using gin (company gin_trgm_ops);
create index jobs_embedding on jobs using hnsw (embedding vector_cosine_ops);

-- The record. Every change to a job, and every action hunt takes, lands here.
create table events (
    id     bigint generated always as identity primary key,
    at     timestamptz not null default now(),
    job_id bigint references jobs (id) on delete cascade,
    kind   text not null,
    body   text not null,
    data   jsonb not null default '{}'
);
create index events_at on events (at desc);
create index events_job on events (job_id, at);
create index events_kind on events (kind, at desc);
