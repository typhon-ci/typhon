// @generated automatically by Diesel CLI.

diesel::table! {
    builds (build_id) {
        build_id -> Integer,
        build_hash -> Text,
        build_drv -> Text,
        build_status -> Text,
    }
}

diesel::table! {
    evaluations (evaluation_id) {
        evaluation_id -> Integer,
        evaluation_jobset -> Integer,
        evaluation_num -> Integer,
        evaluation_locked_flake -> Text,
        evaluation_time_created -> BigInt,
        evaluation_actions_path -> Text,
        evaluation_status -> Text,
    }
}

diesel::table! {
    jobs (job_id) {
        job_evaluation -> Integer,
        job_id -> Integer,
        job_name -> Text,
        job_build -> Integer,
        job_status -> Text,
    }
}

diesel::table! {
    jobsets (jobset_id) {
        jobset_project -> Integer,
        jobset_id -> Integer,
        jobset_name -> Text,
        jobset_flake -> Text,
    }
}

diesel::table! {
    projects (project_id) {
        project_id -> Integer,
        project_name -> Text,
        project_title -> Text,
        project_description -> Text,
        project_homepage -> Text,
        project_decl -> Text,
        project_decl_locked -> Text,
        project_actions_path -> Text,
        project_key -> Text,
    }
}

diesel::joinable!(evaluations -> jobsets (evaluation_jobset));
diesel::joinable!(jobs -> builds (job_build));
diesel::joinable!(jobs -> evaluations (job_evaluation));
diesel::joinable!(jobsets -> projects (jobset_project));

diesel::allow_tables_to_appear_in_same_query!(
    builds,
    evaluations,
    jobs,
    jobsets,
    projects,
);
