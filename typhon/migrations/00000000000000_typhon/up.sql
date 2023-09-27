CREATE TABLE projects (
    project_id INTEGER NOT NULL PRIMARY KEY,
    project_actions_path TEXT,
    project_description TEXT DEFAULT "" NOT NULL,
    project_homepage TEXT DEFAULT "" NOT NULL,
    project_key TEXT NOT NULL,
    project_legacy BOOL NOT NULL,
    project_name TEXT NOT NULL,
    project_title TEXT DEFAULT "" NOT NULL,
    project_url TEXT DEFAULT "" NOT NULL,
    project_url_locked TEXT DEFAULT "" NOT NULL,
    UNIQUE(project_name)
);

CREATE TABLE jobsets (
    jobset_id INTEGER NOT NULL PRIMARY KEY,
    jobset_legacy BOOL NOT NULL,
    jobset_name TEXT NOT NULL,
    jobset_project INTEGER NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
    jobset_url TEXT NOT NULL,
    UNIQUE(jobset_project, jobset_name)
);

CREATE TABLE evaluations (
    evaluation_id INTEGER NOT NULL PRIMARY KEY,
    evaluation_actions_path TEXT,
    evaluation_jobset INTEGER NOT NULL REFERENCES jobsets(jobset_id) ON DELETE CASCADE,
    evaluation_num INTEGER NOT NULL,
    evaluation_status TEXT NOT NULL CHECK(evaluation_status in ('pending', 'success', 'error', 'canceled')),
    evaluation_time_created BIGINT NOT NULL,
    evaluation_time_finished BIGINT,
    evaluation_url_locked TEXT NOT NULL,
    UNIQUE(evaluation_jobset, evaluation_num)
);

CREATE TABLE jobs (
    job_id INTEGER NOT NULL PRIMARY KEY,
    job_begin_status TEXT CHECK(job_begin_status in ('pending', 'success', 'error', 'canceled')) NOT NULL,
    job_begin_time_finished BIGINT,
    job_begin_time_started BIGINT,
    job_build_drv TEXT NOT NULL,
    job_build_out TEXT NOT NULL,
    job_build_status TEXT CHECK(job_build_status in ('pending', 'success', 'error', 'canceled')) NOT NULL,
    job_build_time_finished BIGINT,
    job_build_time_started BIGINT,
    job_dist BOOLEAN NOT NULL,
    job_end_status TEXT CHECK(job_end_status in ('pending', 'success', 'error', 'canceled')) NOT NULL,
    job_end_time_finished BIGINT,
    job_end_time_started BIGINT,
    job_evaluation INTEGER NOT NULL REFERENCES evaluations(evaluation_id) ON DELETE CASCADE,
    job_name TEXT NOT NULL,
    job_system TEXT NOT NULL,
    job_time_created BIGINT NOT NULL,
    UNIQUE(job_evaluation, job_system, job_name)
);

CREATE TABLE logs (
    log_id INTEGER NOT NULL PRIMARY KEY,
    log_evaluation INTEGER REFERENCES evaluations(evaluation_id) ON DELETE CASCADE,
    log_job INTEGER REFERENCES jobs(job_id) ON DELETE CASCADE,
    log_stderr TEXT NOT NULL,
    log_type TEXT NOT NULL CHECK(log_type in ('eval', 'begin', 'end')),
    UNIQUE(log_evaluation, log_job, log_type)
);
