{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };
  outputs = { self, nixpkgs, crane, flake-utils, advisory-db, ... }:
    flake-utils.lib.eachSystem [ flake-utils.lib.system.x86_64-linux ] (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;
        src = craneLib.cleanCargoSource ./.;
        commonArgs = {
          inherit src;
          strictDeps = true;
          buildInputs = [ ];
          # Additional environment variables can be set directly
          # MY_CUSTOM_VAR = "some value";
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        macsmc-zonepower = craneLib.buildPackage (
          commonArgs
          // {
            doCheck = false;
            inherit cargoArtifacts;
            meta = {
              mainProgram = "macsmc-zonepower";
            };
          }
        );
      in
      {
        checks = {
          inherit macsmc-zonepower;
          macsmc-zonepower-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            }
          );
          macsmc-zonepower-doc = craneLib.cargoDoc (
            commonArgs
            // {
              inherit cargoArtifacts;
              # This can be commented out or tweaked as necessary, e.g. set to
              # `--deny rustdoc::broken-intra-doc-links` to only enforce that lint
              env.RUSTDOCFLAGS = "--deny warnings";
            }
          );
          macsmc-zonepower-fmt = craneLib.cargoFmt {
            inherit src;
          };
          macsmc-zonepower-toml-fmt = craneLib.taploFmt {
            src = pkgs.lib.sources.sourceFilesBySuffices src [ ".toml" ];
            # taplo arguments can be further customized below as needed
            # taploExtraArgs = "--config ./taplo.toml";
          };
          # Audit dependencies
          macsmc-zonepower-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };
          # Audit licenses
          macsmc-zonepower-deny = craneLib.cargoDeny {
            inherit src;
          };
          # Run tests with cargo-nextest
          macsmc-zonepower-nextest = craneLib.cargoNextest (
            commonArgs
            // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
              cargoNextestPartitionsExtraArgs = "--no-tests=pass";
            }
          );
        };
        packages = {
          default = macsmc-zonepower;
        };
        apps.default = flake-utils.lib.mkApp {
          drv = macsmc-zonepower;
        };
        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          # Additional dev-shell environment variables can be set directly
          # MY_CUSTOM_DEVELOPMENT_VAR = "something else";
          packages = [ ];
        };
      }
    )
    // {
      nixosModules.macsmc-zonepower = { config, lib, pkgs, ... }: {
        options.services.macsmc-zonepower = {
          enable = lib.mkEnableOption "Enables macsmc-zonepower service";
          package = lib.mkOption {
            type = lib.types.package;
            default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
          };
        };
        config =
          let
            cfg = config.services.macsmc-zonepower;
          in
          lib.mkIf cfg.enable {
            systemd.services.macsmc-zonepower = {
              description = "Apply SMC reported zone power limits";
              documentation = [ "https://github.com/sharpenedblade/macsmc-zonepower" ];
              wantedBy = [ "multi-user.target" ];
              unitConfig = {
                StartLimitIntervalSec = 120;
                StartLimitBurst = 10;
              };
              serviceConfig = {
                Type = "exec";
                ExecStart = "${cfg.package}/bin/macsmc-zonepower";
                Restart = "always";
                RestartSec = "2s";
                CapabilityBoundingSet = "";
                AmbientCapabilities = "";
                NoNewPrivileges = "yes";
                SystemCallFilter = [ "@system-service" "~@aio @chown @io-event @ipc @keyring @memlock @network-io @resources @setuid @sync" ];
                SystemCallErrorNumber = "EPERM";
                SystemCallArchitectures = "native";
                LockPersonality = "yes";
                MemoryDenyWriteExecute = "yes";
                RestrictNamespaces = "yes";
                RestrictRealtime = "yes";
                RestrictSUIDSGID = "yes";
                PrivateTmp = "yes";
                PrivateDevices = "yes";
                PrivateNetwork = "yes";
                DevicePolicy = "closed";
                RestrictAddressFamilies = "none";
                ProtectSystem = "strict";
                ProtectHome = "yes";
                ProtectProc = "invisible";
                ProtectClock = "yes";
                ProtectHostname = "yes";
                ProtectKernelLogs = "yes";
                ProtectKernelModules = "yes";
                ProtectControlGroups = "yes";
                KeyringMode = "private";
                RemoveIPC = "yes";
              };
            };
          };
      };
    };
}
