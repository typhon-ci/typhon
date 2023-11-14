# Concepts

## Note

Core concepts are described with a flake workflow in mind. But Typhon also
supports a more traditional workflow, see at the end of this section for
details.

## Overview

Projects are the central abstraction of Typhon. A project typically corresponds
to an under CI repository. Projects define jobsets, which in turn spawn jobs.
Jobsets typically correspond to branches of the repository. They are evaluated
periodically, typically on push events. These evaluations produce the Nix jobs
associated with a commit.

On top of these concepts, taken from Hydra, Typhon adds *actions*. Actions are
user-defined scripts, triggered by Typhon on certain occasions. They can have
different purposes, like triggering evaluations, creating new jobsets, setting
statuses or deploying something.

## Projects

Projects are defined declaratively. This means that almost no configuration is
made in Typhon, everything is done externally via a Nix flake. Concretely, a
project is defined by a flake URL (`github:typhon-ci/typhon` for instance). The
referenced flake must expose an output `typhonProject` defining the project
settings.

`typhonProject` contains two attributes: `meta` and `actions`. `meta` is an
attribute set which defines metadata about the project: a title, a description
and a homepage. `actions` is an attribute set of derivations that build actions
for the project and holds encrypted secrets for use by the actions.

A project typically configures CI for a repository, but the declaration can
exist in a separate repository. In fact, the declaration of a project is quite
sensitive since it defines the way the project's unencrypted secrets are
handled. Malicious edits to the declaration can potentially leak these secrets.

## Jobsets

A jobset is also a flake URL (`github:typhon-ci/typhon/main`), referencing a
flake that exposes an output `typhonJobs`. `typhonJobs` is an attribute set of
derivations, called jobs, that are built by Typhon. Jobsets typically correspond
to the branches of the repository. Their flake URL is locked
periodically, creating an evaluation.

Jobsets updates and evaluations are meant to be triggered automatically by
the `webhook` action.

## Evaluations

An evaluation locks the flake URL of a jobset
(`github:typhon-ci/typhon/606cc3b9517038e38f782126c02d305c9bdeb87e`). It
typically corresponds to a commit on the repository. Once the jobset is locked,
the output `typhonJobs` is evaluated and the corresponding jobs are spawned.

## Jobs

Jobs are the result of an evaluation, there is one for each derivation defined
in the jobset. A job run consists of the build of the derivation and the
execution of two actions, one at the beginning and one at the end. These actions
are typically used to set statuses on the commit or to do deployment.

## Actions

Actions are scripts run by Typhon in isolation from the system, but connected to
the internet. They play different roles in Typhon. At the moment there are four
actions a project can define:

- The `jobsets` action is responsible for declaring the jobsets of a project.
  It is triggered periodically by the `webhook` action, typically when a branch
  is created on the repository.

- The `begin` and `end` actions are run at the beginning and end of all jobs of
  your project. They are typically used to set statuses on your repository, but
  can also be used for deployment.

- The `webhook` action is triggered by calls to a specific endpoint of the API.
  It outputs commands for Typhon to update or evaluate jobsets. It is meant to
  trigger jobs automatically.

Actions can also expose a `secrets` file. This is an age encrypted JSON file
that typically contains tokens for the actions. It must be encrypted with the
project's public key and is decrypted at runtime and passed as input to the
actions.

Thanks to the use of actions, Typhon is forge-agnostic: it has no code specific
to any forge. Instead, it is the actions' job to plug Typhon to the user's
workflow. The actions can be built using the Nix library that comes with Typhon.

## Legacy mode

In legacy mode, flake URLs are still used to declare projects and jobsets, but
the underlying expressions do not need to be flakes. Instead of the output
`typhonProject`, a legacy project must expose the expression `nix/typhon.nix`,
that will produce the same content as `typhonProject`. Similarly, a legacy
jobset must expose `nix/jobs.nix` instead of `typhonJobs`. These expressions are
functions called without any arguments, and must evaluate purely.
