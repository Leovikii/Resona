#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
set -euo pipefail
: "${RESONA_AUR_SSH_KEY:?AUR SSH credential is required when AUR publishing is enabled}"
: "${RESONA_AUR_KNOWN_HOSTS:?Pinned aur.archlinux.org host key is required}"
: "${RESONA_RELEASE_TAG:?Release tag is required}"
recipe_dir=$(realpath "$1")
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT
umask 077
printf '%s\n' "$RESONA_AUR_SSH_KEY" > "$work_dir/key"
printf '%s\n' "$RESONA_AUR_KNOWN_HOSTS" > "$work_dir/known_hosts"
export GIT_SSH_COMMAND="ssh -i $work_dir/key -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$work_dir/known_hosts"
field() { sed -n "s/^[[:space:]]*$2 = //p" "$1" | head -n1; }
package_name=$(field "$recipe_dir/.SRCINFO" pkgbase)
case "$package_name" in resona-preview-bin|resona-bin) ;; *) echo 'Unexpected AUR package' >&2; exit 1 ;; esac
# Check the public fixed URL and checksum before changing AUR; do not execute a remote recipe.
asset_url=$(field "$recipe_dir/.SRCINFO" source)
case "$asset_url" in "https://github.com/Leovikii/Resona/releases/download/$RESONA_RELEASE_TAG/"*.tar.gz) ;; *) echo 'Release URL mismatch' >&2; exit 1 ;; esac
curl --fail --location --retry 4 --max-time 180 "$asset_url" -o "$work_dir/payload.tar.gz"
printf '%s  %s\n' "$(field "$recipe_dir/.SRCINFO" sha256sums)" "$work_dir/payload.tar.gz" | sha256sum --check --status
git clone "ssh://aur@aur.archlinux.org/$package_name.git" "$work_dir/aur"
new_version="$(field "$recipe_dir/.SRCINFO" pkgver)-$(field "$recipe_dir/.SRCINFO" pkgrel)"
if [[ -f "$work_dir/aur/.SRCINFO" ]]; then
  old_version="$(field "$work_dir/aur/.SRCINFO" pkgver)-$(field "$work_dir/aur/.SRCINFO" pkgrel)"
  comparison=$(vercmp "$new_version" "$old_version")
  if (( comparison < 0 )); then echo "Refusing AUR downgrade: $old_version -> $new_version" >&2; exit 1; fi
  if (( comparison == 0 )); then
    cmp "$recipe_dir/PKGBUILD" "$work_dir/aur/PKGBUILD"
    cmp "$recipe_dir/.SRCINFO" "$work_dir/aur/.SRCINFO"
    echo 'AUR already contains this exact recipe'
    exit 0
  fi
fi
cp "$recipe_dir/PKGBUILD" "$recipe_dir/.SRCINFO" "$work_dir/aur/"
git -C "$work_dir/aur" -c user.name='Resona release workflow' -c user.email='actions@users.noreply.github.com' add PKGBUILD .SRCINFO
git -C "$work_dir/aur" -c user.name='Resona release workflow' -c user.email='actions@users.noreply.github.com' commit -m "Release $new_version ($RESONA_RELEASE_TAG)"
git -C "$work_dir/aur" push origin HEAD:master
