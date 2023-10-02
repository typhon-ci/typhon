CREATE TABLE projects (
    actions_path TEXT,
    description TEXT DEFAULT '' NOT NULL,
    flake BOOL NOT NULL,
    homepage TEXT DEFAULT '' NOT NULL,
    id INTEGER NOT NULL PRIMARY KEY,
    key TEXT NOT NULL,
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
    id INTEGER NOT NULL PRIMARY KEY,
    jobset_id INTEGER NOT NULL REFERENCES jobsets (id),
    log_id INTEGER NOT NULL REFERENCES logs (id),
    num BIGINT NOT NULL,
    status TEXT NOT NULL CHECK (status in ('pending', 'success', 'error', 'canceled')),
    time_created BIGINT NOT NULL,
    time_finished BIGINT,
    url TEXT NOT NULL,
    UNIQUE (jobset_id, num)
);

CREATE TABLE jobs (
    begin_log_id INTEGER NOT NULL REFERENCES logs (id),
    begin_status TEXT NOT NULL CHECK (begin_status in ('pending', 'success', 'error', 'canceled')),
    begin_time_finished BIGINT,
    begin_time_started BIGINT,
    build_drv TEXT NOT NULL,
    build_out TEXT NOT NULL,
    build_status TEXT NOT NULL CHECK (build_status in ('pending', 'success', 'error', 'canceled')),
    build_time_finished BIGINT,
    build_time_started BIGINT,
    dist BOOL NOT NULL,
    end_log_id INTEGER NOT NULL REFERENCES logs (id),
    end_status TEXT NOT NULL CHECK (end_status in ('pending', 'success', 'error', 'canceled')),
    end_time_finished BIGINT,
    end_time_started BIGINT,
    evaluation_id INTEGER NOT NULL REFERENCES evaluations (id),
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    system TEXT NOT NULL,
    time_created BIGINT NOT NULL,
    UNIQUE (evaluation_id, system, name)
);

CREATE TABLE logs (
    id INTEGER NOT NULL PRIMARY KEY,
    stderr TEXT
);
