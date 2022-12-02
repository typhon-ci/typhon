// @generated automatically by Diesel CLI.

diesel::table! {
    builds (build_id) {
        build_id -> Integer,
        build_drv -> Text,
        build_hash -> Text,
        build_status -> Text,
    }
}

diesel::table! {
    evaluations (evaluation_id) {
        evaluation_id -> Integer,
        evaluation_actions_path -> Nullable<Text>,
        evaluation_jobset -> Integer,
        evaluation_locked_flake -> Text,
        evaluation_num -> Integer,
        evaluation_status -> Text,
        evaluation_time_created -> BigInt,
    }
}

diesel::table! {
    jobs (job_id) {
        job_id -> Integer,
        job_build -> Integer,
        job_evaluation -> Integer,
        job_name -> Text,
        job_status -> Text,
    }
}

diesel::table! {
    jobsets (jobset_id) {
        jobset_id -> Integer,
        jobset_flake -> Text,
        jobset_name -> Text,
        jobset_project -> Integer,
    }
}

diesel::table! {
    projects (project_id) {
        project_id -> Integer,
        project_actions_path -> Nullable<Text>,
        project_decl -> Text,
        project_decl_locked -> Text,
        project_description -> Text,
        project_homepage -> Text,
        project_key -> Text,
        project_name -> Text,
        project_title -> Text,
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
