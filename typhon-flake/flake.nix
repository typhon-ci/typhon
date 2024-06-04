{
  inputs.x.flake = false;
  outputs =
    { x, ... }:
    {
      typhonJobs =
        if builtins.pathExists "${x}/nix/jobs.nix" then import "${x}/nix/jobs.nix" { } else null;
      typhonProject =
        if builtins.pathExists "${x}/nix/typhon.nix" then import "${x}/nix/typhon.nix" { } else null;
    };
}
