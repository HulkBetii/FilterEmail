# VPS Deploy Key Rotation

This runbook covers safe rotation of the local deploy SSH key at `.codex_deploy/filteremail_vps_key`.

## Current production target

- SSH target: `root@103.245.237.81`
- SSH port: `22`
- Active canonical key path: `.codex_deploy/filteremail_vps_key`
- Active key fingerprint: `SHA256:51F3WGmhnSY+8MQJZrtxHZpwNQqatZMBhyNuRn9xerU`
- Retired key backup path: `.codex_deploy/filteremail_vps_key.retired-2026-04-17`

Current tracked fingerprint reference:

- old key: `SHA256:zcM8BLG510yZK53/uMtbNecD7QFBlXRcu4WADSCj6ZM`

The repository intentionally does not automate this change because the target host may be stable while `authorized_keys`, local CI usage, and rollout timing still live outside source control.

## Preconditions

- Identify the exact remote target in advance. The current production target is `root@103.245.237.81`.
- Keep one known-good SSH session open until the new key is verified.
- Confirm `.codex_deploy/` stays gitignored during the whole rotation.
- Confirm any CI, local scripts, or SSH config that reference the old key path.

## Recommended rotation flow

1. Generate a replacement keypair next to the old one without overwriting it yet.

```bash
ssh-keygen -t ed25519 \
  -f .codex_deploy/filteremail_vps_key.next \
  -C "filteremail deploy 2026-04-17"
```

2. Copy the new public key to the server while the old key still works.

```bash
ssh-copy-id -i .codex_deploy/filteremail_vps_key.next.pub -p 22 root@103.245.237.81
```

If `ssh-copy-id` is unavailable, append the public key manually to `~/.ssh/authorized_keys` on the server and ensure permissions stay strict:

```bash
chmod 700 ~/.ssh
chmod 600 ~/.ssh/authorized_keys
```

3. Verify login explicitly with the new key before changing anything else.

```bash
ssh -i .codex_deploy/filteremail_vps_key.next -p 22 root@103.245.237.81
```

4. Run one real deploy or one harmless remote command with the new key to confirm the full path works.

```bash
ssh -i .codex_deploy/filteremail_vps_key.next -p 22 root@103.245.237.81 'hostname && whoami'
```

5. Update local SSH config, deployment scripts, or CI secrets to use the new key path.

6. Remove the old public key from `authorized_keys` only after step 5 is confirmed.

7. Archive or securely delete the old private key, then optionally rename the new key into the canonical filename:

```bash
mv .codex_deploy/filteremail_vps_key.next .codex_deploy/filteremail_vps_key
mv .codex_deploy/filteremail_vps_key.next.pub .codex_deploy/filteremail_vps_key.pub
chmod 600 .codex_deploy/filteremail_vps_key
chmod 644 .codex_deploy/filteremail_vps_key.pub
```

## Validation checklist

- `ssh -i <new-key> -p 22 root@103.245.237.81` succeeds from a fresh terminal.
- The deployment command succeeds with the new key.
- No script or config still points at the retired key file.
- The old public key is removed from the server.
- The old private key is removed from local machines and secret stores.

## Notes

- Do not delete the old key first.
- The current server-side `authorized_keys` was reduced to the new key during the 2026-04-17 rotation, so any remaining machine that still uses the retired key must be updated before it can deploy again.
- Use [verify-vps-deploy.md](verify-vps-deploy.md) as the companion runbook for validating a real deploy after key rotation.
