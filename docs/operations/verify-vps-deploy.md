# Verify VPS Deploy Runbook

This is the current production deploy runbook for the SMTP verification service.

## Current server layout

- SSH target: `root@103.245.237.81`
- SSH port: `22`
- Runtime service: `verify-vps.service`
- Runtime binary: `/usr/local/bin/verify-vps`
- Environment file: `/etc/filteremail/verify-vps.env`
- Source checkout on server: `/opt/filteremail/verify-vps`
- Runtime user/group: `verifyvps:verifyvps`
- Server architecture: `x86_64`
- OS kernel observed during validation: `Linux 6.8.0-31-generic`

Important:

- The running service does **not** build from source on the VPS.
- The VPS currently does **not** have a Rust toolchain installed.
- Deploy by building a Linux `x86_64` binary elsewhere, then copying that binary to the server.

## Authentication

Use the deploy key stored at:

```bash
.codex_deploy/filteremail_vps_key
```

Do not store or document the server password in repository files. SSH key auth is the source of truth for deployment.

## Preflight checks

Confirm access and inspect the current service before replacing anything:

```bash
ssh -i .codex_deploy/filteremail_vps_key -p 22 root@103.245.237.81 'hostname && whoami'
ssh -i .codex_deploy/filteremail_vps_key -p 22 root@103.245.237.81 'systemctl is-active verify-vps.service'
ssh -i .codex_deploy/filteremail_vps_key -p 22 root@103.245.237.81 'systemctl cat verify-vps.service'
ssh -i .codex_deploy/filteremail_vps_key -p 22 root@103.245.237.81 'stat -c "%n | %U:%G | %a | %s bytes" /usr/local/bin/verify-vps /etc/filteremail/verify-vps.env'
```

## Build the release binary

Build a Linux `x86_64` release artifact from the repository root.

Example:

```bash
cargo build --release --manifest-path verify-vps/Cargo.toml --target x86_64-unknown-linux-gnu
```

If you are building from macOS or another non-Linux host, use your existing cross-compile toolchain or CI workflow to produce the same `x86_64-unknown-linux-gnu` binary.

Expected artifact:

```bash
verify-vps/target/x86_64-unknown-linux-gnu/release/verify-vps
```

## Safe deploy flow

1. Upload the new binary to a temporary location on the server.

```bash
scp -i .codex_deploy/filteremail_vps_key -P 22 \
  verify-vps/target/x86_64-unknown-linux-gnu/release/verify-vps \
  root@103.245.237.81:/tmp/verify-vps.new
```

2. Backup the current binary and install the new one with the same executable permissions.

```bash
ssh -i .codex_deploy/filteremail_vps_key -p 22 root@103.245.237.81 '
  cp /usr/local/bin/verify-vps /usr/local/bin/verify-vps.bak-$(date +%F-%H%M%S) &&
  install -o root -g root -m 755 /tmp/verify-vps.new /usr/local/bin/verify-vps &&
  rm -f /tmp/verify-vps.new
'
```

3. Restart the service and inspect the result immediately.

```bash
ssh -i .codex_deploy/filteremail_vps_key -p 22 root@103.245.237.81 '
  systemctl restart verify-vps.service &&
  systemctl status verify-vps.service --no-pager -l | sed -n "1,20p"
'
```

4. Verify that the service is active and that the binary path is still the expected one.

```bash
ssh -i .codex_deploy/filteremail_vps_key -p 22 root@103.245.237.81 '
  systemctl is-active verify-vps.service &&
  systemctl show -p ExecMainStatus verify-vps.service &&
  ls -l /usr/local/bin/verify-vps
'
```

## Post-deploy checks

Use these checks after every deploy:

```bash
ssh -i .codex_deploy/filteremail_vps_key -p 22 root@103.245.237.81 '
  systemctl is-active verify-vps.service &&
  journalctl -u verify-vps.service -n 50 --no-pager
'
```

If you know the public endpoint and API key, also run one real `POST /verify/smtp/v2` smoke request from a trusted machine after restart.

## Rollback

If the new binary fails, restore the most recent backup and restart the service:

```bash
ssh -i .codex_deploy/filteremail_vps_key -p 22 root@103.245.237.81 '
  ls -1t /usr/local/bin/verify-vps.bak-* | head -n 1
'
```

Then replace the binary with that backup and restart:

```bash
ssh -i .codex_deploy/filteremail_vps_key -p 22 root@103.245.237.81 '
  latest=$(ls -1t /usr/local/bin/verify-vps.bak-* | head -n 1) &&
  install -o root -g root -m 755 "$latest" /usr/local/bin/verify-vps &&
  systemctl restart verify-vps.service &&
  systemctl status verify-vps.service --no-pager -l | sed -n "1,20p"
'
```

## Notes

- `/opt/filteremail/verify-vps` is the server-side source checkout, not the runtime binary path.
- `verify-vps.service` runs as `verifyvps`, but root deploys the binary and controls the service.
- Treat `docs/prompts/` as historical implementation artifacts. This runbook is the operational source of truth for the current VPS.
