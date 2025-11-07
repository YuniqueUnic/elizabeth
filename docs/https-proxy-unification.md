# Eliminating Mixed Content with a Unified Next.js Proxy

## TL;DR

- Keep every browser-visible API call on the same HTTPS origin by setting
  `NEXT_PUBLIC_API_URL=/api/v1`.
- Route the real backend traffic exclusively through the Next.js server by
  wiring `INTERNAL_API_URL=http://elizabeth-backend:4092/api/v1`.
- Rebuild the frontend image so the generated JavaScript no longer contains
  absolute `http://` URLs, then redeploy the stack and verify `/api/v1` is
  served via `https://box.yunique.top`.

## Why Mixed Content Happened

The public site `https://box.yunique.top` terminates TLS at a Baota-managed
Nginx, tunnels through frp, and finally reaches the `elizabeth-frontend`
container on port 4001. Inside that container the browser bundle still called
`http://elizabeth-backend:4092`, because `.env.docker` exported an absolute
origin into `NEXT_PUBLIC_API_URL`. Modern browsers refuse to downgrade an HTTPS
page to HTTP for security reasons, so every API call was blocked before it
reached the reverse proxy chain.

## Unifying the Frontend and Backend

1. **Expose only a relative API path to the browser** Update `.env.docker` (or
   the `.env` used by your CI) to pin `NEXT_PUBLIC_API_URL=/api/v1`. Relative
   paths inherit the page origin, so requests become
   `https://box.yunique.top/api/v1/...` automatically. At the same time add
   `INTERNAL_API_URL=http://elizabeth-backend:4092/api/v1` so that the Next.js
   server knows where to forward the traffic across the Docker bridge network.
   > Reference: `.env.docker` lines 17-25.

2. **Let Next.js handle the proxy hop** `web/next.config.mjs` already rewrites
   `/api/v1/:path*` to `INTERNAL_API_URL`. Once the runtime receives a relative
   request, it transparently performs the hop inside the container network,
   meaning the backend stays private and no browser sees plain HTTP endpoints.

3. **Rebuild and redeploy the frontend**
   ```bash
   # Build a clean image so the baked JS bundles inherit the new env vars
   docker compose build --no-cache frontend

   # Restart the containers
   docker compose up -d frontend
   ```
   This ensures that any previously generated chunks referencing
   `http://elizabeth-backend` are replaced with `/api/v1`.

4. **Keep the app origin accurate** Set
   `NEXT_PUBLIC_APP_URL=https://box.yunique.top` for production so server-side
   rendering and generated links always reference HTTPS. Local workflows can
   continue to use `http://localhost:4001` via overrides.

## Verification Checklist

- `curl -I http://localhost:4001/api/v1/health` inside the frontend container
  returns `200`, confirming the rewrite works before TLS termination.
- `curl -I https://box.yunique.top/api/v1/health` from the public edge returns
  the same headers, proving `/api/v1` is now available over HTTPS.
- Browser DevTools → Network shows every API request targeting
  `https://box.yunique.top/api/v1/...` with no Mixed Content warnings in the
  console.
- `docker compose logs frontend` contains the startup line
  `Next.js: INTERNAL_API_URL from env: http://elizabeth-backend:4092/api/v1`,
  verifying the env var is detected.

## Rollback Plan

Should you need to revert quickly, switch the frontend image tag back to the
previous version or restore the old `.env.docker` values, then
`docker compose up -d frontend`. The reverse proxy stack is untouched by this
change, so rollback risk is limited to the frontend container.

## Lessons Learned

- Favor relative API paths for browsers whenever the backend already sits behind
  the same origin. It removes an entire class of TLS and cookie issues.
- Keep “public” and “internal” URLs split: `NEXT_PUBLIC_API_URL` is for the
  browser, `INTERNAL_API_URL` is for the server. Mixing them leaks
  infrastructure details and causes Mixed Content.
- Regenerate static assets whenever environment variables influence build
  output—otherwise stale bundles keep serving deprecated origins.

Following these steps collapses the frontend and backend traffic onto one HTTPS
surface without touching the FRP or Nginx topology, delivering a simple,
KISS-compliant fix for the Mixed Content incident.
