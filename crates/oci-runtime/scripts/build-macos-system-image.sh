#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/build-system-image.sh
source "$script_directory/lib/build-system-image.sh"

export K_SYSTEM_IMAGE_SCHEMA_VERSION='koci.macos-system-image.v1'
export K_SYSTEM_IMAGE_ARCHITECTURE='aarch64'
export K_SYSTEM_IMAGE_ELF_PATTERN='ELF 64-bit.*ARM aarch64.*statically linked'
export K_SYSTEM_IMAGE_ALPINE_URL='https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/aarch64/alpine-minirootfs-3.22.5-aarch64.tar.gz'
export K_SYSTEM_IMAGE_ALPINE_SHA256='3fbc6285032ed46821b511292633d7b2a6306a2e254f590e92bdafff56cf2f70'
export K_SYSTEM_IMAGE_ALPINE_SIZE=3966256
export K_SYSTEM_IMAGE_FILESYSTEM_UUID='a3a30c1a-2026-4000-8000-000000000001'
export K_SYSTEM_IMAGE_DIRECTORY_HASH_SEED='a3a30c1a-2026-4000-8000-000000000002'
runtime_assets="$script_directory/../crates/kbox/runtime/runtime-assets.json"
[[ -f "$runtime_assets" && ! -L "$runtime_assets" ]] || { echo "runtime assets must be a plain file" >&2; exit 1; }
K_SYSTEM_IMAGE_RUNTIME_JSON="$(jq --compact-output --exit-status '
  def one_file($role): [.files[] | select(.role == $role)]
    | if length == 1 then .[0].sha256 else error("runtime role must resolve exactly once") end;
  if .schema_version != "koci.kbox-runtime-assets.v1" then error("unexpected runtime asset schema") else . end
  | [.bundles[] | select(.target_os == "macos" and .target_arch == "aarch64")]
  | if length == 1 then .[0] else error("macOS arm64 runtime must resolve exactly once") end
  | if .kernel.guest_load_address != 2147483648 or .kernel.entry_address != 2147483648
    then error("unsupported macOS kernel load or entry address") else . end
  | {archive_sha256: .archive_sha256, libkrun_sha256: one_file("library"),
     firmware_sha256: one_file("firmware"), kernel_bundle_size: .kernel.size,
     kernel_bundle_sha256: .kernel.sha256,
     kernel_guest_load_address: "0x0000000080000000", kernel_entry_address: "0x0000000080000000"}
' "$runtime_assets")"

export K_SYSTEM_IMAGE_RUNTIME_JSON

kbuild_system_image "$@"
