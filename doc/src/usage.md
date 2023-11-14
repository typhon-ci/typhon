# Usage

This section gives an example of how to use Typhon with a GitHub project. Let's
assume your username is `$user` and you have two repositories,
`github.com/$user/$project` and `github.com/$user/$config`. `$project` is the
repository you want to put under CI, `$config` is going to contain the Typhon
declaration. These two repositories can actually be the same, but separating the
two can mitigate security concerns. Finally, let's assume your Typhon instance
URL is `$typhon_url` (you must have https enabled).

## Creating a new Typhon project

Log in to your Typhon instance and create a new project, with an identifier
`$id` (typically `$id == $project`). Set the declaration to use the flake URL
`github:$user/$config`. Once the project is created, a public key is associated
to it, let's call it `$pk`.

## GitHub settings

We need to generate a token on GitHub and make sure it has permission to update
statuses on `$project`, let's call it `$token`. Then let's generate a random
string `$secret` and add a webhook to `$project` with the following settings:
- payload URL: `$typhon_url/api/projects/$id/webhook`
- content type: `application/json`
- secret: `$secret`
- events: Just the `push` event

## The configuration flake

Let's create a flake in the `$config` repository, then add an output
`typhonProject`. We are going to import `typhon` as a flake input and use the
`mkGithubProject` helper function from the library:

```nix
{
  inputs = {typhon.url = "github:typhon-ci/typhon";};

  outputs = {
    self,
    typhon,
  }: {
    typhonProject = typhon.lib.github.mkGithubProject {
      owner = "$user";
      repo = "$project";
      secrets = ./secrets.age;
      typhon_url = "$typhon_url";
    };
  };
}
```

We need to generate the `secrets.age` file. First let's write a `secrets.json`
file containing the secrets you generated (don't commit it!):

```json
{
  "github_token": "$token",
  "github_webhook_secret": "$secret"
}
```

Then, we encrypt the JSON file with `age`, using the public key of the project:

```shell
nix run nixpkgs#age -- --encrypt -r "$pk" -o secrets.age secrets.json
```

We also need to generate the lock file:

```shell
nix flake lock
```

Finally, we commit `secrets.age`, `flake.nix` and `flake.lock`.

## The project flake

In the `$project` repository, we create a flake with a `typhonJobs` attribute.
For instance, let's declare GNU hello as your only job:

```nix
{
  inputs = {nixpkgs.url = "nixpkgs";};

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in {
    typhonJobs.${system} = {
      inherit (pkgs) hello;
    };
  };
}
```

We need to generate a lock file and commit `flake.nix` and `flake.lock`.

## Refreshing the project declaration

Let's go to your project's page on Typhon and refresh the declaration. This is
not done automatically on purpose. Always be careful before refreshing: if a
malicious commit was made on `$config`, your secrets could be compromised. Once
this is done, your Typhon project is using the settings declared in `$config`.

## Verifying everything is working

We can now update the jobsets of your project from the project interface. A list
of jobsets should appear, one for each branch of your repository. Now, any push
to the repository should generate an evaluation in the corresponding jobset and
statuses should appear on your repository.
