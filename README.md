# commut

commut is a personal terminal you host on your own server and access through your own domain.

It is designed for personal use: one server, one domain, one authorized key pair, and a simple browser-based client.

## Requirements

Before installing commut, prepare:

- a server
- a domain pointed at that server
- Nix on the target machine
- a Linux environment with user-level `systemd`
- an authorization key pair

## Quick Start

### 1. Generate a Key Pair

Run:

```sh
mkdir -p ~/.config/commut
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 \
  -out       ~/.config/commut/authorized.pri.pem \
  -outpubkey ~/.config/commut/authorized.pub.pem
```

This creates:

- `~/.config/commut/authorized.pri.pem` as the client-side private key
- `~/.config/commut/authorized.pub.pem` as the server-side authorized public key

Keep `authorized.pri.pem` on the client side. The server needs `authorized.pub.pem`.

### 2. Install commut

Copy the public key to the server if needed, then run:

```sh
nix run github:hooreique/commut#commut-installer -- install \
  --domain your-domain.example.com \
  --authorized-pubkey-file ~/.config/commut/authorized.pub.pem
```

The installer sets up the commut backend and a Caddy-based entrypoint for your domain.

### 3. Set Up the Browser Client

Open `https://your-domain.example.com/app/story.html`.

That page is the browser-side key setup entrypoint. Paste the contents of `authorized.pri.pem`, choose a short passphrase, and save it.

Then open `https://your-domain.example.com/app/index.html` and connect from the main app.

## Notes

- The installer expects an authorized public key file and will fail if it does not exist.
- The default public key location used by commut is `~/.config/commut/authorized.pub.pem`.
- commut sets `COMMUT=1` inside hosted terminal sessions. Check this variable when a shell or editor needs to detect that it is running under commut.
- If you need installer options, run `nix run github:hooreique/commut#commut-installer -- install --help`.
- To remove an installation, run `nix run github:hooreique/commut#commut-installer -- uninstall`.
