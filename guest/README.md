# Sandbox guest assets

`kernel/` holds the debranded Linux guest kernel sources and patches used by
the fused sandbox engine (Spec050).

`out/` is **local build output** (gitignored): `Image` / `bzImage`,
`initramfs.cpio`, and the sparse `data.img` template. Point the engine at it
with `KBOX_SANDBOX_GUEST_DIR=$REPO/guest/out`, or place `guest/` at the repo
root so `guest_dir()` discovers `guest/out` from a checkout binary.

Build helpers: `scripts/build-sandbox-guest.sh` (disk / kernel / initramfs).
