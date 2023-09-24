# Usage

This section gives an example of how to use Typhon with a GitHub project. Let's
assume your username is "user" and you have two repositories, "project" and
"config". "project" is the repository you want to put under CI, "config" is
going to contain the Typhon declaration. These two repositories can actually be
the same.

## Generate a GitHub token

Generate a token on GitHub and make sure it has permission to update statuses on
the "project" repository.

## Create a project

Log in to your Typhon instance and create a new project. As the declaration, use
the flake URI `github:user/config`. Once the project is created, a public key
is associated to it.

## Declare a project

Create a flake in the "config" repository, then add a `typhonProject` attribute.
In practice, you can rely on Typhon's library to declare projects. Here, you can
use the `mkGithubProject` helper function:

```nix
{
  inputs = { typhon.url = "github:typhon-ci/typhon"; };

  outputs = { self, typhon }:
    {
      typhonProject = typhon.lib.github.mkGithubProject {
        owner = "user";
        repo = "project";
        secrets = ./secrets.age;
      };
    };
}
```

The `secrets.age` file must be encrypted with the public key of the project you
created on Typhon. It contains a JSON object with your GitHub token:

```json
{
  "github_token": "..."
}
```

Encrypt the JSON file with `age`:

```shell
nix run nixpkgs#age -- --encrypt -r $public_key -o secrets.age secrets.json
```

Finally, you need to commit the lock file of your flake.

## Declare jobs

In the "project" repository, create a flake with a `typhonJobs` attribute.
For instance, you can declare GNU hello as your only job:

```nix
{
  inputs = { nixpkgs.url = "nixpkgs"; };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      typhonJobs.${system} = {
        inherit (pkgs) hello;
      };
    };
}
```

You also need to commit the lock file in this repository.

## Refresh

Go to your project's page on Typhon and refresh the declaration. This is not
done automatically on purpose. Always be careful before refreshing, if a
malicious commit was made on "config" this could compromise your secrets.

## Update jobsets

You should now be able to update the jobsets of your project. A list of jobsets
should appear, one for each branch of your repository. At the moment this is not
done automatically, at some point it should be done either periodically by
Typhon or through the use of webhooks.

## Evaluate a jobset

Go to a jobset's page and run an evaluation, it should appear promptly. Do this
after each commit on the corresponding branch. This too should be automatic in
the future.

## You're done

By going to the evaluation's page you will see the evaluation's status. Once it
is finished and successful, you will be able to browse your jobs. Statuses
should appear on your commits on GitHub.
