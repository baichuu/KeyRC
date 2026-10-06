#!/usr/bin/env bash
set -euo pipefail

project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
data_home=${XDG_DATA_HOME:-"$HOME/.local/share"}
cache_home=${XDG_CACHE_HOME:-"$HOME/.cache"}
binary_dir=${KEYRC_BIN_DIR:-"$HOME/.local/bin"}
binary="$binary_dir/keyrc"
desktop_dir="$data_home/applications"
desktop_file="$desktop_dir/keyrc.desktop"
icon_root="$data_home/icons/hicolor"
icon_dir="$icon_root/1024x1024/apps"
icon_file="$icon_dir/keyrc.png"
build=true
start=true

for argument in "$@"; do
  case "$argument" in
  --no-build) build=false ;;
  --no-start) start=false ;;
  *)
    echo "Unknown option: $argument" >&2
    echo "Usage: $0 [--no-build] [--no-start]" >&2
    exit 2
    ;;
  esac
done

if $build; then
  cargo build --manifest-path "$project_dir/Cargo.toml" --release --locked
fi

release_binary="$project_dir/target/release/keyrc"
if [[ ! -x "$release_binary" ]]; then
  echo "Release binary not found: $release_binary" >&2
  exit 1
fi

mkdir -p "$binary_dir" "$desktop_dir" "$icon_dir"
install -m 755 "$release_binary" "$binary.new"
mv -f "$binary.new" "$binary"
install -m 644 "$project_dir/assets/keyrc.png" "$icon_file"

desktop_tmp=$(mktemp "$desktop_dir/.keyrc.desktop.XXXXXX")
trap 'rm -f "$desktop_tmp"' EXIT
cat >"$desktop_tmp" <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=KeyRC
Comment=Display pressed keys in a desktop overlay
Exec=$binary
Icon=keyrc
Terminal=false
Categories=Utility;Accessibility;
StartupNotify=false
EOF
chmod 644 "$desktop_tmp"
mv -f "$desktop_tmp" "$desktop_file"
trap - EXIT

if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$desktop_file"
fi
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$desktop_dir"
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache --force --ignore-theme-index "$icon_root" >/dev/null 2>&1 || true
fi

if $start && [[ -n ${DISPLAY:-} ]]; then
  mkdir -p "$cache_home"
  mapfile -t running_pids < <(pgrep -x keyrc || true)
  if ((${#running_pids[@]})); then
    kill "${running_pids[@]}"
    for _ in {1..50}; do
      alive=false
      for pid in "${running_pids[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
          alive=true
          break
        fi
      done
      $alive || break
      sleep 0.1
    done
  fi
  setsid --fork "$binary" </dev/null >"$cache_home/keyrc.log" 2>&1
fi

printf 'Installed KeyRC:\n  binary: %s\n  desktop: %s\n  icon: %s\n' \
  "$binary" "$desktop_file" "$icon_file"
