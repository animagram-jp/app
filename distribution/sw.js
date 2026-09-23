const VERSION = "{version}";
const PRECACHE = [
    "./",
    "./init.js?v={version}",
    "./manifest.json",
    "./worker.js?v={version}",
    "./app/app_bg.wasm?v={version}",
    "./app/app.js?v={version}",
    "./css/css/reset.css?v={version}",
    "./css/css/base.css?v={version}",
    "./css/css/data_style.css?v={version}",
    "./css/css/data_size.css?v={version}",
    "./css/css/data_sign.css?v={version}",
    "./css/css/data_group.css?v={version}",
    "./css/css/input.css?v={version}",
    "./css/css/button.css?v={version}",
    "./css/style.css?v={version}",
    "./font/IBMPlexSans-Regular.woff2?v={version}",
    "./font/IBMPlexSans-SemiBold.woff2?v={version}",
    "./image/animagram.png?v={version}",
];

self.addEventListener("install", (e) => {
    e.waitUntil(
        caches.open(VERSION).then(c =>
            Promise.allSettled(PRECACHE.map(url => c.add(url)))
        ).then(results => {
            results.forEach((r, i) => {
                if (r.status === "rejected") {
                    console.warn(`sw: precache failed for ${PRECACHE[i]}`, r.reason);
                }
            });
        }).then(() => self.skipWaiting())
    );
});

self.addEventListener("activate", (e) => {
    e.waitUntil(
        caches.keys()
        .then(keys => Promise.all(keys.filter(k => k !== VERSION).map(k => caches.delete(k))))
        .then(() => self.clients.claim())
    );
});

function response_cross_origin_isolation(res) {
    const headers = new Headers(res.headers);
    headers.set("Cross-Origin-Opener-Policy", "same-origin");
    headers.set("Cross-Origin-Embedder-Policy", "require-corp");
    return new Response(res.body, {
        status: res.status,
        statusText: res.statusText,
        headers,
    });
}

const PRECACHE_URLS = new Set(PRECACHE.map((u) => new URL(u, self.location.href).href));

self.addEventListener("fetch", (e) => {
    const req = e.request;
    if (req.method !== "GET") return;

    const url = new URL(req.url);
    if (url.origin !== self.location.origin) return;

    const is_precached = PRECACHE_URLS.has(url.href) ||
        (req.mode === "navigate" && PRECACHE_URLS.has(new URL("./", self.location.href).href));
    if (!is_precached) return;

    if (req.mode === "navigate") {
        e.respondWith(
            fetch(req).then((raw_res) => {
                const res = response_cross_origin_isolation(raw_res);
                const copy = res.clone();
                e.waitUntil(caches.open(VERSION).then((c) => c.put(req, copy)));
                return res;
            }).catch(() => caches.match(req).then((r) => r ?? caches.match("./")))
        );
        return;
    }

    e.respondWith(
    caches.match(req).then((hit) => {
      if (hit) return hit;
      return fetch(req).then((raw_res) => {
        const res = response_cross_origin_isolation(raw_res);
        if (res.ok) {
          const copy = res.clone();
          e.waitUntil(caches.open(VERSION).then((c) => c.put(req, copy)));
        }
        return res;
      });
    })
  );
});

self.addEventListener("message", (e) => {
    if (e.data?.type !== "PREFETCH") return;
    const requester = e.source;
    e.waitUntil(
    caches.open(VERSION).then(async (c) => {
        const list = e.data.urls ?? [];
        let done = 0;
        for (const u of list) {
            try {
                await c.add(u);
            } catch (_) {}
            done += 1;
            requester?.postMessage({ type: "PREFETCH_PROGRESS", done, total: list.length });
        }
    })
    );
});
