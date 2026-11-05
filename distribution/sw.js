const VERSION = "{version}";
const SITES = {
    "./calendar/": [
        "./calendar/",
        "./calendar/app/app_bg.wasm?v={version}",
        "./calendar/app/app.js?v={version}",
        "./calendar/calendar.css?v={version}",
        "./calendar/data/calendar.json",
    ],
    "./coc7th/": [
        "./coc7th/",
        "./coc7th/app/app_bg.wasm?v={version}",
        "./coc7th/app/app.js?v={version}",
        "./coc7th/coc7th.css?v={version}",
    ],
    "./": [
        "./",
        "./init.js?v={version}",
        "./js/style.js?v={version}",
        "./manifest.json",
        "./worker.js?v={version}",
        "./css/import.css?v={version}",
        "./css/library/reset.css?v={version}",
        "./css/library/base.css?v={version}",
        "./css/library/data_style.css?v={version}",
        "./css/library/data_size.css?v={version}",
        "./css/library/data_sign.css?v={version}",
        "./css/library/data_group.css?v={version}",
        "./css/library/input.css?v={version}",
        "./css/library/button.css?v={version}",
        "./css/library/checkbox.css?v={version}",
        "./css/library/toggle.css?v={version}",
        "./css/library/radio.css?v={version}",
        "./css/library/heading.css?v={version}",
        "./css/library/list.css?v={version}",
        "./css/library/search.css?v={version}",
        "./css/library/table.css?v={version}",
        "./css/library/slider.css?v={version}",
        "./css/library/step.css?v={version}",
        "./css/library/popup.css?v={version}",
        "./css/library/error.css?v={version}",
        "./css/library/toast.css?v={version}",
        "./css/style.css?v={version}",
        "./font/IBMPlexSans-Regular.woff2?v={version}",
        "./font/IBMPlexSans-SemiBold.woff2?v={version}",
        "./image/animagram.png?v={version}",
    ],
};

const PRECACHE = Object.values(SITES).flat();
const PRECACHE_PATHS = new Set(PRECACHE.map((u) => new URL(u, self.location.href).pathname));
const BASES = Object.keys(SITES)
    .map((base) => new URL(base, self.location.href).pathname)
    .sort((a, b) => b.length - a.length);

function base_of(pathname) {
    return BASES.find((base) => pathname.startsWith(base)) ?? "/";
}

self.addEventListener("install", (e) => {
    e.waitUntil(
        caches.open(VERSION).then(c =>
            Promise.allSettled(PRECACHE.map(url => c.add(url)))
        ).then(results => {
            const failed = results.map((r, i) => r.status === "rejected" ? PRECACHE[i] : null).filter(Boolean);
            results.forEach((r, i) => {
                if (r.status === "rejected") {
                    console.warn(`sw: precache failed for ${PRECACHE[i]}`, r.reason);
                }
            });
            if (failed.length > 0) throw new Error(`sw: precache incomplete: ${failed.join(", ")}`);
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

self.addEventListener("fetch", (e) => {
    const req = e.request;
    if (req.method !== "GET") return;

    const url = new URL(req.url);
    if (url.origin !== self.location.origin) return;

    const is_navigate = req.mode === "navigate";
    const is_worker = req.destination === "worker" || req.destination === "sharedworker";
    if (!is_navigate && !is_worker && !PRECACHE_PATHS.has(url.pathname)) return;

    if (is_navigate) {
        e.respondWith(
            fetch(req).then((raw_res) => {
                if (raw_res.type === "opaqueredirect") return raw_res;
                const res = response_cross_origin_isolation(raw_res);
                const copy = res.clone();
                e.waitUntil(caches.open(VERSION).then((c) => c.put(req, copy)));
                return res;
            }).catch(() => {
                const base = BASES.find((base) => `${url.pathname}/` === base);
                if (base) return Response.redirect(new URL(`${base}${url.search}`, self.location.href), 307);
                return caches.match(req).then((r) => r ?? caches.match(`.${base_of(url.pathname)}`)).then((r) => r && response_cross_origin_isolation(r));
            })
        );
        return;
    }

    e.respondWith(
    caches.match(req).then((hit) => {
      if (hit) return response_cross_origin_isolation(hit);
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
