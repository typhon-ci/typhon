CREATE TABLE projects (
    project_id INTEGER NOT NULL PRIMARY KEY,
    project_actions_path TEXT,
    project_decl TEXT DEFAULT "" NOT NULL,
    project_decl_locked TEXT DEFAULT "" NOT NULL,
    project_description TEXT DEFAULT "" NOT NULL,
    project_homepage TEXT DEFAULT "" NOT NULL,
    project_key TEXT NOT NULL,
    project_name TEXT NOT NULL,
    project_title TEXT DEFAULT "" NOT NULL,
    UNIQUE(project_name)
);

CREATE TABLE jobsets (
    jobset_id INTEGER NOT NULL PRIMARY KEY,
    jobset_flake TEXT NOT NULL,
    jobset_name TEXT NOT NULL,
    jobset_project INTEGER NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
    UNIQUE(jobset_project, jobset_name)
);

CREATE TABLE evaluations (
    evaluation_id INTEGER NOT NULL PRIMARY KEY,
    evaluation_actions_path TEXT,
    evaluation_flake_locked TEXT NOT NULL,
    evaluation_jobset INTEGER NOT NULL REFERENCES jobsets(jobset_id) ON DELETE CASCADE,
    evaluation_num INTEGER NOT NULL,
    evaluation_status TEXT NOT NULL CHECK(evaluation_status in ('pending', 'success', 'error', 'canceled')),
    evaluation_time_created BIGINT NOT NULL,
    UNIQUE(evaluation_jobset, evaluation_num)
);

CREATE TABLE jobs (
    job_id INTEGER NOT NULL PRIMARY KEY,
    job_build INTEGER NOT NULL REFERENCES builds(build_id) ON DELETE CASCADE,
    job_evaluation INTEGER NOT NULL REFERENCES evaluations(evaluation_id) ON DELETE CASCADE,
    job_name TEXT NOT NULL,
    job_status TEXT CHECK(job_status in ('begin', 'waiting', 'end', 'success', 'error', 'canceled')) NOT NULL,
    UNIQUE(job_evaluation, job_name)
);

CREATE TABLE builds (
    build_id INTEGER NOT NULL PRIMARY KEY,
    build_drv TEXT NOT NULL UNIQUE,
    build_hash TEXT NOT NULL UNIQUE,
    build_status TEXT NOT NULL CHECK(build_status in ('pending', 'success', 'error', 'canceled'))
);

CREATE TABLE logs (
    log_id INTEGER NOT NULL PRIMARY KEY,
    log_evaluation INTEGER REFERENCES evaluations(evaluation_id) ON DELETE CASCADE,
    log_job INTEGER REFERENCES jobs(job_id) ON DELETE CASCADE,
    log_stderr TEXT NOT NULL,
    log_type TEXT NOT NULL CHECK(log_type in ('build', 'evaluation', 'job_begin', 'job_end')),
    UNIQUE(log_evaluation, log_job, log_type)
);
