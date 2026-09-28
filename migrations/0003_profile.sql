-- What hunt knows about your work: repos, your CV, notes. Personal data is
-- stripped before anything is written to `summary` or `facts`.
create table notes (
    id         bigint generated always as identity primary key,
    source     text not null,               -- repo | cv | note
    path       text not null,
    title      text not null,
    body       text not null,
    summary    text,
    facts      jsonb,
    updated_at timestamptz not null default now(),
    embedding  vector(256),
    search     tsvector generated always as (
        setweight(to_tsvector('english', title), 'A') ||
        setweight(to_tsvector('english', coalesce(summary, '')), 'B') ||
        setweight(to_tsvector('english', left(body, 20000)), 'C')
    ) stored,
    unique (source, path)
);
create index notes_search on notes using gin (search);
create index notes_embedding on notes using hnsw (embedding vector_cosine_ops);

-- One row: the profile hunt builds from your notes and your decisions.
create table profile (
    id         integer primary key default 1 check (id = 1),
    data       jsonb not null,
    updated_at timestamptz not null default now()
);
