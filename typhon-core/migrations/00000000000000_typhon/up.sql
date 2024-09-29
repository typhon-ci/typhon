CREATE TABLE projects (
    actions_path TEXT,
    description TEXT DEFAULT '' NOT NULL,
    flake BOOL NOT NULL,
    homepage TEXT DEFAULT '' NOT NULL,
    id INTEGER NOT NULL PRIMARY KEY,
    key TEXT NOT NULL,
    last_refresh_task_id INTEGER REFERENCES tasks (id),
    name TEXT NOT NULL,
    title TEXT DEFAULT '' NOT NULL,
    url TEXT DEFAULT '' NOT NULL,
    url_locked TEXT DEFAULT '' NOT NULL,
    UNIQUE (name)
);

CREATE TABLE jobsets (
    flake BOOL NOT NULL,
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    project_id INTEGER NOT NULL REFERENCES projects (id),
    url TEXT NOT NULL,
    UNIQUE (project_id, name)
);

CREATE TABLE evaluations (
    actions_path TEXT,
    flake BOOL NOT NULL,
    id INTEGER NOT NULL PRIMARY KEY,
    jobset_name TEXT NOT NULL,
    project_id INTEGER NOT NULL REFERENCES projects (id),
    task_id INTEGER NOT NULL REFERENCES tasks (id),
    time_created BIGINT NOT NULL,
    url TEXT NOT NULL,
    uuid TEXT NOT NULL,
    UNIQUE (uuid)
);

CREATE TABLE jobs (
    dist BOOL NOT NULL,
    drv TEXT NOT NULL,
    evaluation_id INTEGER NOT NULL REFERENCES evaluations (id),
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    out TEXT NOT NULL,
    system TEXT NOT NULL,
    tries INTEGER NOT NULL,
    UNIQUE (evaluation_id, system, name)
);

CREATE TABLE runs (
    begin_id INTEGER REFERENCES actions (id),
    build_id INTEGER REFERENCES builds (id),
    end_id INTEGER REFERENCES actions (id),
    id INTEGER NOT NULL PRIMARY KEY,
    job_id INTEGER NOT NULL REFERENCES jobs (id),
    num INTEGER NOT NULL,
    time_created BIGINT NOT NULL,
    UNIQUE (job_id, num)
);

CREATE TABLE builds (
    drv TEXT NOT NULL,
    id INTEGER NOT NULL PRIMARY KEY,
    task_id INTEGER NOT NULL REFERENCES tasks (id),
    time_created BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    UNIQUE (uuid)
);

CREATE TABLE actions (
    id INTEGER NOT NULL PRIMARY KEY,
    input TEXT NOT NULL,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    project_id INTEGER NOT NULL REFERENCES projects (id),
    task_id INTEGER NOT NULL REFERENCES tasks (id),
    time_created BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    UNIQUE (uuid)
);

CREATE TABLE tasks (
    id INTEGER NOT NULL PRIMARY KEY,
    log_id INTEGER NOT NULL REFERENCES logs (id),
    status INTEGER NOT NULL,
    time_finished BIGINT,
    time_started BIGINT
);

CREATE TABLE logs (
    id INTEGER NOT NULL PRIMARY KEY,
    stderr TEXT
);

CREATE TABLE users (
    fullname TEXT,
    id INTEGER NOT NULL PRIMARY KEY,
    password BLOB NOT NULL,
    username TEXT NOT NULL,
    unique (username)
);

CREATE TABLE sessions (
    expires_at BIGINT,
    id INTEGER NOT NULL PRIMARY KEY,
    last_used_at BIGINT,
    name TEXT NOT NULL,
    revoked_at BIGINT,
    token BLOB NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users (id),
    unique (user_id, name)
);

CREATE TABLE permissions (
    id INTEGER NOT NULL PRIMARY KEY,
    type TEXT NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users (id)
);
