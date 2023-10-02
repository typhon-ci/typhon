// @generated automatically by Diesel CLI.

diesel::table! {
    evaluations (id) {
        actions_path -> Nullable<Text>,
        id -> Integer,
        jobset_id -> Integer,
        log_id -> Integer,
        num -> BigInt,
        status -> Text,
        time_created -> BigInt,
        time_finished -> Nullable<BigInt>,
        url -> Text,
    }
}

diesel::table! {
    jobs (id) {
        begin_log_id -> Integer,
        begin_status -> Text,
        begin_time_finished -> Nullable<BigInt>,
        begin_time_started -> Nullable<BigInt>,
        build_drv -> Text,
        build_out -> Text,
        build_status -> Text,
        build_time_finished -> Nullable<BigInt>,
        build_time_started -> Nullable<BigInt>,
        dist -> Bool,
        end_log_id -> Integer,
        end_status -> Text,
        end_time_finished -> Nullable<BigInt>,
        end_time_started -> Nullable<BigInt>,
        evaluation_id -> Integer,
        id -> Integer,
        name -> Text,
        system -> Text,
        time_created -> BigInt,
    }
}

diesel::table! {
    jobsets (id) {
        flake -> Bool,
        id -> Integer,
        name -> Text,
        project_id -> Integer,
        url -> Text,
    }
}

diesel::table! {
    logs (id) {
        id -> Integer,
        stderr -> Nullable<Text>,
    }
}

diesel::table! {
    projects (id) {
        actions_path -> Nullable<Text>,
        description -> Text,
        flake -> Bool,
        homepage -> Text,
        id -> Integer,
        key -> Text,
        name -> Text,
        title -> Text,
        url -> Text,
        url_locked -> Text,
    }
}

diesel::joinable!(evaluations -> jobsets (jobset_id));
diesel::joinable!(evaluations -> logs (log_id));
diesel::joinable!(jobs -> evaluations (evaluation_id));
diesel::joinable!(jobsets -> projects (project_id));

diesel::allow_tables_to_appear_in_same_query!(evaluations, jobs, jobsets, logs, projects,);
