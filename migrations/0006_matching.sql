-- Trained weights, by name: `like` predicts your approval, `odds` a reply.
create table models (
    name       text primary key,
    data       jsonb not null,
    trained_at timestamptz not null default now()
);
