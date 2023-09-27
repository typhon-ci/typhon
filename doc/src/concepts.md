# Concepts

## Projects

Projects are the central abstraction of Typhon. A project typically corresponds
to an under CI repository. Projects are defined declaratively. This means
almost no configuration is made in Typhon, everything is done externally via a
Nix flake. Concretely, a project is defined by a flake URI. This flake must
expose a `typhonProject` attribute defining the project settings.

`typhonProject` contains two attributes: `meta` and `actions`. `meta` is an
attribute set which defines metadata about your project: a title, a description
and a homepage. `actions` is an attribute set of derivations that build actions
for your project. Actions are another core concept of Typhon and are introduced
below.

## Actions

Actions are scripts called by Typhon for different purposes. At the moment there
are four actions a project can define.

- The `jobsets` action is responsible for declaring jobsets for your project.
  Jobsets, like projects, are flake URIs. Typically they correspond to a branch
  of your repository. These flakes must expose a `typhonJobs` attribute, that in
  turn declares jobs for you project.

- The `begin` and `end` actions are run at the beginning and end of all jobs of
  your project. They are typically used to set statuses on your repository, but
  can also be used for deployment.

- The `webhook` action is triggered by calls to a specific endpoint of the API.
  It outputs commands for Typhon to update jobsets or evaluate a jobset. It is
  meant to trigger jobs automatically if your forge supports webhooks.

Actions can also expose a `secrets` file. This is an age encrypted JSON file
that typically contains tokens for the actions. It must be encrypted with the
project's public key and is decrypted at runtime and passed as input to the
actions.

## Jobsets

A Typhon jobset is a flake that exposes a `typhonJobs` attribute. `typhonJobs`
is an attribute set of derivations that are built by Typhon. Jobsets typically
follow your repository branches. Jobsets are evaluated periodically.

## Evaluations

An evaluation locks the flake URI of a jobset. It typically corresponds to a
commit on your repository. Once the flake is locked, the `typhonJobs` attribute
is evaluated and the corresponding derivations are built.

## Jobs

Jobs are derivations defined in `typhonJobs`. A job run consists of the `begin`
action, the derivation build, and the `end` action.
