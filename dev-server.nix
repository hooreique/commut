{
  fonts,
  runtimeInputs,
  writeShellApplication,
}:

writeShellApplication {
  name = "dev-server";
  inherit runtimeInputs;

  text = ''
    set -euo pipefail

    usage() {
      cat <<EOF
    dev-server

    Run a local commut development server.

    Usage:
      nix run .#dev-server
      nix run .#dev-server -- --help

    Environment:
      COMMUT_DEV_ROOT           repository root (default: current directory)
      COMMUT_DEV_HOST           Caddy listen host (default: 127.0.0.1)
      COMMUT_DEV_PORT           Caddy listen port (default: 8080)
      COMMUT_DEV_BACKEND_HOST   backend listen host (default: COMMUT_HOST or 127.0.0.1)
      COMMUT_DEV_BACKEND_PORT   backend listen port (default: COMMUT_PORT or 3000)

    The backend keeps its normal authorized-key behavior:
      COMMUT_AUTHORIZED_PUBLIC_KEY_PEM_FILE
      COMMUT_AUTHORIZED_PUBLIC_KEY_PEM
      ~/.config/commut/authorized.pub.pem
    EOF
    }

    die() {
      echo "dev-server: $*" >&2
      exit 1
    }

    if [ "$#" -gt 0 ]; then
      case "$1" in
        -h|--help)
          usage
          exit 0
          ;;
        *)
          die "unknown argument: $1"
          ;;
      esac
    fi

    repo_root=''${COMMUT_DEV_ROOT:-$PWD}
    repo_root=$(cd "$repo_root" && pwd) || die "repository root does not exist: $repo_root"
    frontend_dir="$repo_root/frontend"
    backend_manifest="$repo_root/backend/Cargo.toml"

    [ -f "$backend_manifest" ] || die "backend/Cargo.toml not found under $repo_root"
    [ -f "$frontend_dir/package.json" ] || die "frontend/package.json not found under $repo_root"

    if [ ! -d "$frontend_dir/node_modules" ]; then
      die "frontend/node_modules is missing.

    Install frontend dependencies first:
      nix develop --command -- pnpm --dir frontend install --frozen-lockfile"
    fi

    if [ -z "''${HOME:-}" ]; then
      die "\$HOME must be set"
    fi

    if [ ! -x "$HOME/.nix-profile/bin/zsh" ]; then
      echo "dev-server: warning: required hosted shell not found at \$HOME/.nix-profile/bin/zsh" >&2
    fi

    dev_host=''${COMMUT_DEV_HOST:-127.0.0.1}
    dev_port=''${COMMUT_DEV_PORT:-8080}
    backend_host=''${COMMUT_HOST:-127.0.0.1}
    backend_host=''${COMMUT_DEV_BACKEND_HOST:-$backend_host}
    backend_port=''${COMMUT_PORT:-3000}
    backend_port=''${COMMUT_DEV_BACKEND_PORT:-$backend_port}

    font_dir="${fonts}/share/fonts"
    mkdir -p "$frontend_dir/public/fonts"
    cp -f "$font_dir"/*.woff2 "$frontend_dir/public/fonts/"

    echo "dev-server: building frontend assets" >&2
    pnpm --dir "$frontend_dir" run build

    tmp_dir=$(mktemp -d "''${TMPDIR:-/tmp}/commut-dev-server.XXXXXX")
    caddyfile="$tmp_dir/Caddyfile"
    caddy_config_home="$tmp_dir/xdg-config"
    caddy_data_home="$tmp_dir/xdg-data"
    mkdir -p "$caddy_config_home" "$caddy_data_home"

    cat > "$caddyfile" <<EOF
    {
      admin off
      auto_https off
    }

    http://:$dev_port {
      bind $dev_host

      handle_path /app/* {
        header Cache-Control "no-cache"
        root * "$frontend_dir/build/app"
        file_server
      }

      @public path /favicon.ico /manifest.json /images/* /fonts/*

      handle @public {
        root * "$frontend_dir/public"
        file_server
      }

      handle_path /dist/* {
        header Cache-Control "public, max-age=31536000, immutable"
        root * "$frontend_dir/dist"
        file_server
      }

      handle /api/* {
        reverse_proxy $backend_host:$backend_port
      }

      handle /sockets/* {
        reverse_proxy $backend_host:$backend_port
      }

      handle {
        respond "404 Not Found" 404
      }
    }
    EOF
    caddy fmt --overwrite "$caddyfile" >/dev/null

    backend_pid=""
    caddy_pid=""

    # shellcheck disable=SC2329
    stop_process() {
      local pid=$1
      if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
      fi
    }

    # shellcheck disable=SC2329
    cleanup() {
      local status=$?
      trap - EXIT INT TERM
      stop_process "$caddy_pid"
      stop_process "$backend_pid"
      rm -rf "$tmp_dir"
      exit "$status"
    }

    trap cleanup EXIT INT TERM

    echo "dev-server: starting backend on http://$backend_host:$backend_port" >&2
    (
      cd "$repo_root"
      COMMUT_HOST="$backend_host" COMMUT_PORT="$backend_port" \
        cargo run --manifest-path backend/Cargo.toml
    ) &
    backend_pid=$!

    echo "dev-server: starting Caddy on http://$dev_host:$dev_port" >&2
    echo "dev-server: open http://$dev_host:$dev_port/app/index.html" >&2
    XDG_CONFIG_HOME="$caddy_config_home" XDG_DATA_HOME="$caddy_data_home" \
      caddy run --config "$caddyfile" --adapter caddyfile &
    caddy_pid=$!

    set +e
    wait -n "$backend_pid" "$caddy_pid"
    status=$?
    set -e
    exit "$status"
  '';
}
