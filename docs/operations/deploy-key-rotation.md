# VPS Deploy Key Rotation

This runbook covers safe rotation of the local deploy SSH key at `.codex_deploy/filteremail_vps_key`.

Current tracked fingerprint reference:

- old key: `SHA256:zcM8BLG510yZK53/uMtbNecD7QFBlXRcu4WADSCj6ZM`

The repository intentionally does not automate this change because the target host, remote user, and `authorized_keys` location live outside source control.

## Preconditions

- Identify the exact remote target in advance: `user@host`.
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
ssh-copy-id -i .codex_deploy/filteremail_vps_key.next.pub user@host
```

If `ssh-copy-id` is unavailable, append the public key manually to `~/.ssh/authorized_keys` on the server and ensure permissions stay strict:

```bash
chmod 700 ~/.ssh
chmod 600 ~/.ssh/authorized_keys
```

3. Verify login explicitly with the new key before changing anything else.

```bash
ssh -i .codex_deploy/filteremail_vps_key.next user@host
```

4. Run one real deploy or one harmless remote command with the new key to confirm the full path works.

```bash
ssh -i .codex_deploy/filteremail_vps_key.next user@host 'hostname && whoami'
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

- `ssh -i <new-key> user@host` succeeds from a fresh terminal.
- The deployment command succeeds with the new key.
- No script or config still points at the retired key file.
- The old public key is removed from the server.
- The old private key is removed from local machines and secret stores.

## Notes

- Do not delete the old key first.
- Do not rely on the repository alone to discover the deploy host; verify it from operational context.
- If the target host is still unknown, stop after documenting the current fingerprint and collecting the remote access details.
