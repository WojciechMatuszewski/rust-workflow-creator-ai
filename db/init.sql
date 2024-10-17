begin;

create extension if not exists vector;


create table if not exists apps (
    id serial primary key,
    name text not null,
    description text not null
);

create table if not exists actions (
    id serial primary key,
    app_id integer not null references apps(id) on delete cascade,
    name text not null,
    description text not null,
    embedding vector(1536) not null
);

create index if not exists idx_embeddings_embedding on actions USING hnsw (embedding vector_cosine_ops);

commit;
