{
  backendPackage,
  caddy,
  clientPackage,
  installFlakeRef,
  lib,
  pkgs,
  stdenv,
  writeShellApplication,
}:
let
  substitute = import ./mustache.nix;
  topLevelHelp = substitute (builtins.readFile ./top-level-help.txt) {
    inherit installFlakeRef;
  };
  installHelp = substitute (builtins.readFile ./install-help.txt) {
    inherit installFlakeRef;
  };
  uninstallHelp = substitute (builtins.readFile ./uninstall-help.txt) {
    inherit installFlakeRef;
  };
  runtimeBundle = pkgs.runCommand "commut-runtime" { } ''
    mkdir -p "$out"
    ln -s ${clientPackage} "$out/client"
    ln -s ${backendPackage} "$out/backend"
    ln -s ${caddy} "$out/caddy"
  '';
  caddyfile = substitute (builtins.readFile ./Caddyfile.template) {
    clientAppRoot = "$runtime_root/client/build/app";
    clientDistRoot = "$runtime_root/client/dist";
    clientPublicRoot = "$runtime_root/client/public";
  };
in
writeShellApplication {
  name = "commut-installer";
  extraShellCheckFlags = [ "-e" "SC2034" ];

  runtimeInputs = [
    caddy
    pkgs.coreutils
    pkgs.findutils
    pkgs.gnugrep
    pkgs.gnused
    pkgs.nix
  ]
  ++ lib.optionals stdenv.isLinux [
    pkgs.systemd
  ];

  text = ''
set -euo pipefail

default_config_home=''${XDG_CONFIG_HOME:-$HOME/.config}
default_data_home=''${XDG_DATA_HOME:-$HOME/.local/share}
default_state_home=''${XDG_STATE_HOME:-$HOME/.local/state}

usage() {
  cat <<EOF
${topLevelHelp}
EOF
}

usage_install() {
  cat <<EOF
${installHelp}
EOF
}

usage_uninstall() {
  cat <<EOF
${uninstallHelp}
EOF
}

die() {
  echo "commut-installer: $*" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

check_loginctl() {
  require_command loginctl
  loginctl show-user "$USER" --property=Linger --value >/dev/null 2>&1 || die \
    "loginctl is present but cannot query linger state for user \"$USER\".

This installer requires a working loginctl environment."
}

check_systemctl_user() {
  require_command systemctl
  systemctl --user show-environment >/dev/null 2>&1 || die \
    "systemctl --user is not usable for user \"$USER\".

This installer expects a working user-level systemd environment."
}

linger_status() {
  loginctl show-user "$USER" --property=Linger --value | tr -d '\n'
}

fail_linger_disabled() {
  cat >&2 <<EOF
commut-installer: systemd user linger is required.
Current status: Linger=no

Without linger, commut user services may stop after you log out.

Next steps:
  1. Run:
     loginctl enable-linger "$USER"
  2. Then rerun:
     nix run ${installFlakeRef} -- install ...

If you want installer to try it for you, rerun with:
  nix run ${installFlakeRef} -- install --enable-linger ...
EOF
  exit 1
}

ensure_linger_enabled() {
  local status
  status="$(linger_status)"

  case "$status" in
    yes)
      return 0
      ;;
    no)
      fail_linger_disabled
      ;;
    *)
      die "unexpected linger state for user \"$USER\": $status"
      ;;
  esac
}

try_enable_linger() {
  local before after
  before="$(linger_status)"
  if [[ "$before" == "yes" ]]; then
    return 0
  fi

  if ! loginctl enable-linger "$USER"; then
    cat >&2 <<EOF
commut-installer: failed to enable linger for user "$USER".

Tried:
  loginctl enable-linger "$USER"

This system may require a different local policy or additional privileges.
Ask an administrator to enable linger for your user, then rerun:

  nix run ${installFlakeRef} -- install ...
EOF
    exit 1
  fi

  after="$(linger_status)"
  [[ "$after" == "yes" ]] || die \
    "attempted to enable linger, but the resulting state is \"$after\" instead of \"yes\""
}

maybe_disable_linger() {
  local before after
  before="$(linger_status)"
  if [[ "$before" == "no" ]]; then
    echo "commut-installer: linger is already disabled for user \"$USER\"." >&2
    return 0
  fi

  if ! loginctl disable-linger "$USER"; then
    cat >&2 <<EOF
commut-installer: failed to disable linger for user "$USER".

Tried:
  loginctl disable-linger "$USER"

If commut was the reason linger was enabled, you can retry this command manually.
If another service still depends on linger, leave it enabled.
EOF
    exit 1
  fi

  after="$(linger_status)"
  [[ "$after" == "no" ]] || die \
    "attempted to disable linger, but the resulting state is \"$after\" instead of \"no\""
}

require_non_empty() {
  local name="$1"
  local value="$2"
  [[ -n "$value" ]] || die "$name is required"
}

write_file() {
  local path="$1"
  mkdir -p "$(dirname "$path")"
  cat >"$path"
}

ensure_replaceable_gcroot() {
  if [[ -e "$runtime_root" || -L "$runtime_root" ]]; then
    [[ -L "$runtime_root" ]] || die \
      "GC root path exists but is not a symlink: $runtime_root"
    rm -f "$runtime_root"
  fi
}

materialize_gcroot() {
  mkdir -p "$gcroot_dir"
  ensure_replaceable_gcroot
  nix-store --add-root "$runtime_root" --indirect -r ${runtimeBundle} >/dev/null
}

remove_gcroot() {
  if [[ -e "$runtime_root" || -L "$runtime_root" ]]; then
    [[ -L "$runtime_root" ]] || die \
      "GC root path exists but is not a symlink: $runtime_root"
    rm -f "$runtime_root"
  fi

  rmdir "$gcroot_dir" >/dev/null 2>&1 || true
}

materialize_install_env() {
  write_file "$install_env_path" <<EOF
COMMUT_HOST=$listen_host
COMMUT_PORT=$listen_port
COMMUT_AUTHORIZED_PUBLIC_KEY_PEM_FILE=$authorized_pubkey_file
EOF
}

materialize_commut_service() {
  write_file "$commut_service_path" <<EOF
[Unit]
Description=Commut
After=network.target

[Service]
EnvironmentFile=$install_env_path
ExecStart=$runtime_root/backend/bin/commut
Restart=on-failure
RestartSec=2s

[Install]
WantedBy=default.target
EOF
}

materialize_caddy_service() {
  write_file "$caddy_service_path" <<EOF
[Unit]
Description=Caddy for Commut
After=network.target

[Service]
ExecStart=$runtime_root/caddy/bin/caddy run --config $caddyfile_path
Restart=on-failure
RestartSec=2s

[Install]
WantedBy=default.target
EOF
}

materialize_caddyfile() {
  local debug_line="# debug"
  if (( caddy_debug )); then
    debug_line="debug"
  fi

  write_file "$caddyfile_path" <<EOF
${caddyfile}
EOF
}

enable_services() {
  systemctl --user daemon-reload
  systemctl --user enable commut.service caddy.service >/dev/null
  systemctl --user restart commut.service caddy.service
}

stop_and_disable_services() {
  systemctl --user stop commut.service caddy.service >/dev/null 2>&1 || true
  systemctl --user disable commut.service caddy.service >/dev/null 2>&1 || true
  systemctl --user daemon-reload
}

config_home="$default_config_home"
data_home="$default_data_home"
state_home="$default_state_home"
domain=""
authorized_pubkey_file=""
caddy_http_port="80"
caddy_https_port="443"
caddy_debug=0
listen_host="127.0.0.1"
listen_port="3000"
enable_linger=0
disable_linger=0

subcommand=""
if [[ $# -eq 0 ]]; then
  usage
  exit 0
fi

if [[ $# -gt 0 ]]; then
  case "$1" in
    --help|-h)
      usage
      exit 0
      ;;
    install|uninstall)
      subcommand="$1"
      shift
      ;;
    *)
      usage
      exit 1
      ;;
  esac
fi

if [[ $# -gt 0 ]]; then
  case "$1" in
    --help|-h)
      case "$subcommand" in
        install)
          usage_install
          ;;
        uninstall)
          usage_uninstall
          ;;
      esac
      exit 0
      ;;
  esac
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --config-home)
      [[ $# -ge 2 ]] || die "--config-home requires a value"
      config_home="$2"
      shift 2
      ;;
    --data-home)
      [[ $# -ge 2 ]] || die "--data-home requires a value"
      data_home="$2"
      shift 2
      ;;
    --state-home)
      [[ $# -ge 2 ]] || die "--state-home requires a value"
      state_home="$2"
      shift 2
      ;;
    --domain)
      [[ $# -ge 2 ]] || die "--domain requires a value"
      domain="$2"
      shift 2
      ;;
    --authorized-pubkey-file)
      [[ $# -ge 2 ]] || die "--authorized-pubkey-file requires a value"
      authorized_pubkey_file="$2"
      shift 2
      ;;
    --http-port)
      [[ $# -ge 2 ]] || die "--http-port requires a value"
      caddy_http_port="$2"
      shift 2
      ;;
    --https-port)
      [[ $# -ge 2 ]] || die "--https-port requires a value"
      caddy_https_port="$2"
      shift 2
      ;;
    --debug-caddy)
      caddy_debug=1
      shift
      ;;
    --listen-host)
      [[ $# -ge 2 ]] || die "--listen-host requires a value"
      listen_host="$2"
      shift 2
      ;;
    --listen-port)
      [[ $# -ge 2 ]] || die "--listen-port requires a value"
      listen_port="$2"
      shift 2
      ;;
    --enable-linger)
      enable_linger=1
      shift
      ;;
    --disable-linger)
      disable_linger=1
      shift
      ;;
    --help|-h)
      case "$subcommand" in
        install)
          usage_install
          ;;
        uninstall)
          usage_uninstall
          ;;
      esac
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

config_dir="$config_home/commut"
data_dir="$data_home/commut"
state_dir="$state_home/commut"
systemd_user_dir="$config_home/systemd/user"
gcroot_dir="$state_dir/gcroots"
runtime_root="$gcroot_dir/current"
caddyfile_path="$config_dir/Caddyfile"
install_env_path="$config_dir/install.env"
commut_service_path="$systemd_user_dir/commut.service"
caddy_service_path="$systemd_user_dir/caddy.service"

case "$subcommand" in
  install)
    check_loginctl
    check_systemctl_user
    if (( enable_linger )); then
      try_enable_linger
    fi
    ensure_linger_enabled

    require_non_empty "--domain" "$domain"
    require_non_empty "--authorized-pubkey-file" "$authorized_pubkey_file"
    [[ -f "$authorized_pubkey_file" ]] || die \
      "authorized public key file does not exist: $authorized_pubkey_file"

    mkdir -p "$config_dir" "$data_dir" "$state_dir" "$systemd_user_dir"
    materialize_gcroot
    materialize_install_env
    materialize_caddyfile
    materialize_commut_service
    materialize_caddy_service
    enable_services

    cat >&2 <<EOF
commut-installer: install completed.

Resolved XDG homes:
  config: $config_home
  data:   $data_home
  state:  $state_home

Managed files:
  $install_env_path
  $caddyfile_path
  $commut_service_path
  $caddy_service_path
  $runtime_root
EOF
    ;;
  uninstall)
    check_loginctl
    check_systemctl_user
    stop_and_disable_services
    rm -f "$commut_service_path" "$caddy_service_path" "$caddyfile_path" "$install_env_path"
    remove_gcroot

    if (( disable_linger )); then
      maybe_disable_linger
      echo "commut-installer: linger was disabled for user \"$USER\"." >&2
    fi

    cat >&2 <<EOF
commut-installer: uninstall completed.

Resolved XDG homes:
  config: $config_home
  data:   $data_home
  state:  $state_home
EOF
    ;;
esac
  '';

  meta = {
    description = "commut service installer";
    mainProgram = "commut-installer";
    homepage = "https://github.com/hooreique/commut";
  };
}
