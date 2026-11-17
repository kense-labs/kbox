# T5 sandbox `-p` publish

Date: 2026-11-17  
≠Pass

## Observed

`kbox run -d --rm -p 18083:80 alpine … nc -l -p 80` then
`curl http://127.0.0.1:18083/` → `ok`.

Also skips MicroVM rootfs diff baseline for Sandbox route (detach path).

## Explicitly not claimed

Full publish matrix / UDP / IPv6 host binds.
