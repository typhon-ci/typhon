// @generated automatically by Diesel CLI.

diesel::table! {
    evaluations (evaluation_id) {
        evaluation_id -> Integer,
        evaluation_actions_path -> Nullable<Text>,
        evaluation_jobset -> Integer,
        evaluation_num -> Integer,
        evaluation_status -> Text,
        evaluation_time_created -> BigInt,
        evaluation_time_finished -> Nullable<BigInt>,
        evaluation_url_locked -> Text,
    }
}

diesel::table! {
    jobs (job_id) {
        job_id -> Integer,
        job_begin_status -> Text,
        job_begin_time_finished -> Nullable<BigInt>,
        job_begin_time_started -> Nullable<BigInt>,
        job_build_drv -> Text,
        job_build_out -> Text,
        job_build_status -> Text,
        job_build_time_finished -> Nullable<BigInt>,
        job_build_time_started -> Nullable<BigInt>,
        job_dist -> Bool,
        job_end_status -> Text,
        job_end_time_finished -> Nullable<BigInt>,
        job_end_time_started -> Nullable<BigInt>,
        job_evaluation -> Integer,
        job_name -> Text,
        job_system -> Text,
        job_time_created -> BigInt,
    }
}

diesel::table! {
    jobsets (jobset_id) {
        jobset_id -> Integer,
        jobset_legacy -> Bool,
        jobset_name -> Text,
        jobset_project -> Integer,
        jobset_url -> Text,
    }
}

diesel::table! {
    logs (log_id) {
        log_id -> Integer,
        log_evaluation -> Nullable<Integer>,
        log_job -> Nullable<Integer>,
        log_stderr -> Text,
        log_type -> Text,
    }
}

diesel::table! {
    projects (project_id) {
        project_id -> Integer,
        project_actions_path -> Nullable<Text>,
        project_description -> Text,
        project_homepage -> Text,
        project_key -> Text,
        project_legacy -> Bool,
        project_name -> Text,
        project_title -> Text,
        project_url -> Text,
        project_url_locked -> Text,
    }
}

diesel::joinable!(evaluations -> jobsets (evaluation_jobset));
diesel::joinable!(jobs -> evaluations (job_evaluation));
diesel::joinable!(jobsets -> projects (jobset_project));
diesel::joinable!(logs -> evaluations (log_evaluation));
diesel::joinable!(logs -> jobs (log_job));

diesel::allow_tables_to_appear_in_same_query!(evaluations, jobs, jobsets, logs, projects,);
