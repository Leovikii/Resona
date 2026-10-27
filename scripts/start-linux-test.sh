#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Copy alongside bin/resona when assembling a local test package.
set -eu
bundle_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
desktop_data_home=${XDG_DATA_HOME:-"$HOME/.local/share"}
export GDK_BACKEND=wayland
export LD_LIBRARY_PATH="$bundle_dir/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
# GTK and fontconfig must retain the desktop's XDG_CONFIG_HOME.
export XDG_DATA_HOME="$bundle_dir/test-profile/data"
export XDG_CACHE_HOME="$bundle_dir/test-profile/cache"
mkdir -p "$XDG_DATA_HOME" "$XDG_CACHE_HOME"
if [ -d "$desktop_data_home/fonts" ] && [ ! -e "$XDG_DATA_HOME/fonts" ] && [ ! -L "$XDG_DATA_HOME/fonts" ]; then
  ln -s -- "$desktop_data_home/fonts" "$XDG_DATA_HOME/fonts"
fi
exec "$bundle_dir/bin/resona" "$@"
