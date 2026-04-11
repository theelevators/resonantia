# Resonantia Docker Deploy

This runs the web app as a Docker image and serves it on your LAN.

## Prerequisites

- Docker Engine
- Docker Compose plugin (`docker compose`)

## Build + run

From this folder (`/home/theelevators/resonantia`):

```bash
docker compose up --build -d
```

Open on your LAN:

- `http://<your-host-ip>:4173`

If `4173` is already in use, choose another host port:

```bash
RESONANTIA_PORT=4180 docker compose up --build -d
```

Then open:

- `http://<your-host-ip>:4180`

## Logs

```bash
docker compose logs -f resonantia-web
```

## Stop

```bash
docker compose down
```

## Notes

- The app still uses the same gateway URL you set in Settings.
- For web clients outside local dev, your gateway should still send proper CORS headers.
