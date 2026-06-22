# WT Backend

HTTP adapter for `wt-3-innertube`. It exposes the JSON contract used by
`wt-2-ui/src/lib/api.ts`, so the UI can run with `VITE_BACKEND=real`.

## Run Locally

```bash
cd wt-6-backend
cargo run
```

The server listens on `0.0.0.0:8787` by default.

```bash
WT_BACKEND_BIND=127.0.0.1:8787 cargo run
```

Point the UI at it:

```bash
cd ../wt-2-ui
VITE_BACKEND=real VITE_BACKEND_URL=http://127.0.0.1:8787 pnpm dev
```

## Hetzner Deployment

### Coolify

Use the repository root as the build context because this crate depends on
`wt-3-innertube` with a local path dependency.

Recommended Coolify settings:

```text
Build Pack: Dockerfile
Base Directory: /
Dockerfile Location: /wt-6-backend/Dockerfile
Port: 8787
```

Environment variables:

```bash
WT_BACKEND_BIND=0.0.0.0:8787
RUST_LOG=info,tower_http=info
# Optional comma-separated Piped fallback instances.
# Defaults to https://api.piped.private.coffee when unset.
WT_PIPED_INSTANCES=https://api.piped.private.coffee
```

Then point the UI to the public Coolify URL:

```bash
VITE_BACKEND=real
VITE_BACKEND_URL=https://your-coolify-backend-domain.example
```

### Systemd

Build on the server:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config
git clone <your-repo-url> /opt/ytb
cd /opt/ytb/wt-6-backend
cargo build --release
```

Create `/etc/systemd/system/wt-backend.service`:

```ini
[Unit]
Description=WT YouTube backend
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=/opt/ytb/wt-6-backend
ExecStart=/opt/ytb/wt-6-backend/target/release/wt-6-backend
Environment=WT_BACKEND_BIND=127.0.0.1:8787
Environment=RUST_LOG=info,tower_http=info
Restart=always
RestartSec=5
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
```

Start it:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now wt-backend
sudo systemctl status wt-backend
```

Put Nginx/Caddy in front of `127.0.0.1:8787` with HTTPS, then set:

```bash
VITE_BACKEND=real
VITE_BACKEND_URL=https://your-domain.example
```
