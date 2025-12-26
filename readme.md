## Prepare Keys

```bash
mkdir -p ~/.config/commut
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 \
  -out       ~/.config/commut/authorized.pri.pem \
  -outpubkey ~/.config/commut/authorized.pub.pem
```

## Prepare Fonts

Font Source: https://www.nerdfonts.com/font-downloads

```bash
mkdir -p ~/Downloads/Hack
curl --location --output ~/Downloads/Hack.tar.xz \
  https://github.com/ryanoasis/nerd-fonts/releases/download/v3.4.0/Hack.tar.xz
tar -xvf ~/Downloads/Hack.tar.xz -C ~/Downloads/Hack
```

```bash
woff2_compress ~/Downloads/Hack/HackNerdFont-Regular.ttf
woff2_compress ~/Downloads/Hack/HackNerdFont-Italic.ttf
woff2_compress ~/Downloads/Hack/HackNerdFont-Bold.ttf
woff2_compress ~/Downloads/Hack/HackNerdFont-BoldItalic.ttf
```

```bash
cp ~/Downloads/Hack/HackNerdFont-*.woff2 ~/ws/commut/public/fonts/
```

```bash
rm -rf ~/Downloads/Hack ~/Downloads/Hack.tar.xz
```

## Getting Started

### Resolve Dependencies

```bash
nix develop --command pnpm install --frozen-lockfile
```

### Develop

#### full

```bash
{ find src/client -type f
  echo src/base.css
} | entr -r nix develop --command bash -c "pnpm run prepare & pnpm run dev"
```

#### only backend

```bash
nix develop --command pnpm run prepare & nix develop --command pnpm run dev
```

### Preview

```bash
nix develop --command pnpm run build && nix develop --command pnpm run start
```

## Deployment

### Clone

```bash
mkdir -p ~/ws/commut
git clone git@github.com:hooreique/commut.git ~/ws/commut
```

### Prepare Keys on Server

See [Prepare Keys](#prepare-keys).

### Prepare Fonts on Server

See [Prepare Fonts](#prepare-fonts).

### Write Caddyfile

```bash
cp   ~/ws/commut/meta/Caddyfile.template ~/ws/commut/meta/Caddyfile
nvim ~/ws/commut/meta/Caddyfile
```

### Enable Linger

```bash
loginctl show-user $USER | grep Linger
loginctl enable-linger $USER
loginctl show-user $USER | grep Linger
```

### Register Services

```bash
cp ~/ws/commut/meta/commut.service \
   ~/ws/commut/meta/caddy.service ~/.config/systemd/user/
systemctl --user enable commut caddy
systemctl --user status commut caddy
```

### Run (or Refresh) Services

```bash
systemctl --user restart commut caddy
```

### Uninstall

```bash
systemctl --user stop    commut caddy
systemctl --user disable commut caddy
rm ~/.config/systemd/user/commut.service ~/.config/systemd/user/caddy.service
# rm -rf ~/ws/commut
# rm -rf ~/.config/commut
# loginctl disable-linger $USER
# loginctl show-user $USER | grep Linger
```
