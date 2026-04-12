const CACHE_NAME = "resonantia-shell-v3";
const OFFLINE_ASSETS = [
  "/",
  "/manifest.webmanifest",
  "/icon-192.png",
  "/icon-512.png",
  "/apple-touch-icon.png",
  "/favicon.png"
];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches
      .open(CACHE_NAME)
      .then((cache) => cache.addAll(OFFLINE_ASSETS))
      .then(() => self.skipWaiting())
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      const keys = await caches.keys();
      await Promise.all(keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key)));
      await self.clients.claim();
    })()
  );
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET") {
    return;
  }

  event.respondWith(
    (async () => {
      const cache = await caches.open(CACHE_NAME);

      try {
        const response = await fetch(request);
        const sameOrigin = request.url.startsWith(self.location.origin);

        if (sameOrigin && response.ok) {
          cache.put(request, response.clone());
        }

        return response;
      } catch {
        const cached = await cache.match(request);
        if (cached) {
          return cached;
        }

        if (request.mode === "navigate") {
          const fallback = await cache.match("/");
          if (fallback) {
            return fallback;
          }
        }

        return new Response("offline", {
          status: 503,
          statusText: "Service Unavailable"
        });
      }
    })()
  );
});
