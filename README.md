# commut

commut is a personal terminal you host on your server and access through your domain. It uses one authorized key pair and a browser client.

## Requirements

- Nix on the server
- Linux with user-level systemd for `commut service`
- A hosted shell at `~/.nix-profile/bin/zsh`
- A domain and an externally managed Caddy server for browser access

## Install

Generate an authorization key pair:

```sh
mkdir -p ~/.config/commut
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 \
  -out       ~/.config/commut/authorized.pri.pem \
  -outpubkey ~/.config/commut/authorized.pub.pem
```

Keep the private key on the client. Copy only the public key to the server.

### With hooddy 0.2.0

Add the commut input and module to `~/.config/hooddy/flake.nix`. A minimal configuration is:

```nix
{
  inputs.hooddy.url = "github:hooreique/hooddy";
  inputs.commut.url = "github:hooreique/commut";

  outputs = { hooddy, commut, ... }: {
    hooddyConfigurations.default = hooddy.lib.hooddyConfiguration {
      system = "x86_64-linux"; # Use aarch64-linux on ARM64.
      modules = [
        commut.hooddyModules.default
        {
          services.commut.enable = true;
          services.commut.domain = "commut.example.com";
        }
      ];
    };
  };
}
```

From `~/.config/hooddy`, run `nix flake lock` and `hooddy switch`.
hooddy installs the Caddy site and starts `hooddy-commut.service` using the public key at
`~/.config/commut/authorized.pub.pem`. Set `services.commut.port` to change the
loopback port from 3000. To remove the site and backend, disable the module and switch again.

When migrating a standalone installation, uninstall its managed site and service
before switching to avoid duplicate routes or port conflicts. For hooddy 0.1 deployments,
follow [hooddy’s migration guide](https://github.com/hooreique/hooddy/blob/main/CONTRIBUTING.md#moving-from-01).

### With standalone Caddy

Install the CLI into your Nix profile:

```sh
nix profile add github:hooreique/commut#commut
```

Install and start the backend as the target user:

```sh
commut service install \
  --authorized-pubkey-file ~/.config/commut/authorized.pub.pem \
  --enable-linger
commut service start
```

Installation creates a stopped, disabled `commut.service`. Linger must be enabled for installation, start, and restart; `--enable-linger` lets commut enable it.

Generate a site file at a location imported by your Caddy configuration:

```sh
commut site install \
  --target ~/.config/caddy/sites.d/commut.caddy \
  --domain commut.example.com
```

The target is any file path you choose; commut has no dependency on the tool managing Caddy. Import the generated file from your main Caddyfile and apply it using your Caddy manager. Global options, certificates, and Caddy reloads are managed externally.

Both installation commands accept `--listen-host` and `--listen-port`, defaulting to `127.0.0.1:3000`. Set matching values on both sides. Sites can be installed and removed independently of systemd. Each site retains its frontend assets through a Nix GC root; reinstalling updates only commut-managed files.

## Browser setup

Open `https://commut.example.com/app/story.html`, paste the private key, choose a short passphrase, and save. Then open `/app/index.html` and connect.

## Standalone CLI operations

```sh
commut service status
commut service restart
commut service stop
commut service start
commut service uninstall
commut site uninstall --target ~/.config/caddy/sites.d/commut.caddy
```

`start` enables automatic startup; `stop` disables it. Both are repeatable. `restart` also starts a stopped service and preserves its automatic startup setting. `status` passes through systemd's output and exit code.

Service removal preserves your public key and sites. Linger remains enabled unless you pass `--disable-linger` to `service uninstall`. Site removal deletes only the managed site file and its asset roots; apply the change through your Caddy manager.

For foreground operation:

```sh
commut serve
```

This executes `commut-server` directly, preserving signals and exit codes. Configure it with `COMMUT_HOST`, `COMMUT_PORT`, `COMMUT_AUTHORIZED_PUBLIC_KEY_PEM_FILE`, or `COMMUT_AUTHORIZED_PUBLIC_KEY_PEM`. The default key path is `~/.config/commut/authorized.pub.pem`. No service or Caddy files are changed.

The separate Nix packages are `commut-server` and `commut-client`. Run `commut --help` or a subcommand's `--help` for options. Existing installations must be removed before using this CLI; no migration is provided.

commut sets `COMMUT=1` inside hosted terminal sessions.
