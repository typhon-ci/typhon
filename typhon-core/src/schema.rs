// @generated automatically by Diesel CLI.

diesel::table! {
    actions (id) {
        id -> Integer,
        input -> Text,
        name -> Text,
        path -> Text,
        project_id -> Integer,
        task_id -> Integer,
        time_created -> BigInt,
        uuid -> Text,
    }
}

diesel::table! {
    builds (id) {
        drv -> Text,
        id -> Integer,
        task_id -> Integer,
        time_created -> BigInt,
        uuid -> Text,
    }
}

diesel::table! {
    evaluations (id) {
        actions_path -> Nullable<Text>,
        flake -> Bool,
        id -> Integer,
        jobset_name -> Text,
        project_id -> Integer,
        task_id -> Integer,
        time_created -> BigInt,
        url -> Text,
        uuid -> Text,
    }
}

diesel::table! {
    jobs (id) {
        dist -> Bool,
        drv -> Text,
        evaluation_id -> Integer,
        id -> Integer,
        name -> Text,
        out -> Text,
        system -> Text,
        tries -> Integer,
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
        last_refresh_task_id -> Nullable<Integer>,
        name -> Text,
        title -> Text,
        url -> Text,
        url_locked -> Text,
    }
}

diesel::table! {
    runs (id) {
        begin_id -> Nullable<Integer>,
        build_id -> Nullable<Integer>,
        end_id -> Nullable<Integer>,
        id -> Integer,
        job_id -> Integer,
        num -> Integer,
        task_id -> Integer,
        time_created -> BigInt,
    }
}

diesel::table! {
    tasks (id) {
        id -> Integer,
        log_id -> Integer,
        status -> Integer,
        time_finished -> Nullable<BigInt>,
        time_started -> Nullable<BigInt>,
    }
}

diesel::joinable!(actions -> projects (project_id));
diesel::joinable!(actions -> tasks (task_id));
diesel::joinable!(builds -> tasks (task_id));
diesel::joinable!(evaluations -> projects (project_id));
diesel::joinable!(evaluations -> tasks (task_id));
diesel::joinable!(jobs -> evaluations (evaluation_id));
diesel::joinable!(jobsets -> projects (project_id));
diesel::joinable!(projects -> tasks (last_refresh_task_id));
diesel::joinable!(runs -> builds (build_id));
diesel::joinable!(runs -> jobs (job_id));
diesel::joinable!(runs -> tasks (task_id));
diesel::joinable!(tasks -> logs (log_id));

diesel::allow_tables_to_appear_in_same_query!(
    actions,
    builds,
    evaluations,
    jobs,
    jobsets,
    logs,
    projects,
    runs,
    tasks,
);
