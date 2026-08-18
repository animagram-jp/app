const VERSION = "{version}";
const PRECACHE = [
    "./",
    "./init.js",
    "./worker.js",
    "./manifest.json",
    "./icon.png",
    "./app/app_bg.wasm",
    "./app/app.js",
    "./css/config.css",
    "./css/button.css",
    "./css/style.css",
    "./font/IBMPlexSans-Regular.woff2",
    "./font/IBMPlexSans-SemiBold.woff2",
];

self.addEventListener("install", (e) => {
    e.waitUntil(
        caches.open(VERSION).then(c =>
            Promise.allSettled(PRECACHE.map(url => c.add(url)))
        ).then(() => self.skipWaiting())
    );
});

self.addEventListener("activate", (e) => {
    e.waitUntil(
        caches.keys()
        .then(keys => Promise.all(keys.filter(k => k !== VERSION).map(k => caches.delete(k))))
        .then(() => self.clients.claim())
    );
});

self.addEventListener("fetch", (e) => {
    if (e.request.method !== "GET") return;
    e.respondWith(
        fetch(e.request)
        .then(res => {
            if (res.ok) {
                const clone = res.clone();
                caches.open(VERSION).then(c => c.put(e.request, clone));
            }
            return res;
        }).catch(() => caches.match(e.request))
    );
});
