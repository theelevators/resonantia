# Resonantia LAN Deploy

This guide runs the web build on your local network so phones/laptops on the same LAN can test it.

## 1) Build + start LAN server

From the project root:

```bash
npm run deploy:lan
```

This will:

- Build static assets
- Start `vite preview` on `0.0.0.0:4173`
- Keep the gateway proxy route active at `/__gateway_proxy__`

## 2) Find your machine IP

On Linux:

```bash
hostname -I | awk '{print $1}'
```

If it prints `192.168.1.24`, open this from other devices:

- `http://192.168.1.24:4173`

## 3) Configure gateway once in app settings

Set gateway base URL to your gateway host, for example:

- `http://10.12.0.11:8090`

The app will route requests through the local preview proxy first.

## 4) Firewall

Allow inbound TCP `4173` on the machine running Resonantia.

If other devices cannot connect, test from the host machine first:

```bash
curl -i http://127.0.0.1:4173
```

## 5) Stop server

Press `Ctrl+C` in the terminal running `npm run deploy:lan`.

## Optional

For LAN development mode with hot reload:

```bash
npm run dev:lan
```
