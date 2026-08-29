const VERSION = "{version}";
const PRECACHE = [
    "./",
    "./init.js?v={version}",
    "./manifest.json",
    "./worker.js?v={version}",
    "./app/app_bg.wasm?v={version}",
    "./app/app.js?v={version}",
    "./css/config.css?v={version}",
    "./css/button.css?v={version}",
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

function withCoi(res) {
    const headers = new Headers(res.headers);
    headers.set("Cross-Origin-Opener-Policy", "same-origin");
    headers.set("Cross-Origin-Embedder-Policy", "require-corp");
    return new Response(res.body, {
        status: res.status,
        statusText: res.statusText,
        headers,
    });
}

self.addEventListener("fetch", (e) => {
    if (e.request.method !== "GET") return;
    e.respondWith(
        fetch(e.request)
        .then(res => {
            if (res.type === "opaque") return res;
            if (res.ok) {
                const clone = res.clone();
                caches.open(VERSION)
                    .then(c => c.put(e.request, clone))
                    .catch(err => console.warn(`sw: cache put failed for ${e.request.url}`, err));
            }
            return withCoi(res);
        }).catch(() => caches.match(e.request).then(res => res && withCoi(res)))
    );
});
