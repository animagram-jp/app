const CACHE = 'calendar-v1';
const PRECACHE = [
  '/',
  '/init.js',
  '/worker.js',
  '/style.css',
  '/manifest.json',
  '/icon.png',
  '/app/app_bg.wasm',
  '/app/app.js',
  'manifest/state.yml',
  '/config.css',
  '/dads-button.css',
  '/style.css',
  '/fonts/IBMPlexSans-Regular.woff2',
  '/fonts/IBMPlexSans-SemiBold.woff2',
];

self.addEventListener('install', (e) => {
  e.waitUntil(
    caches.open(CACHE).then(c =>
      Promise.allSettled(PRECACHE.map(url => c.add(url)))
    ).then(() => self.skipWaiting())
  );
});

self.addEventListener('activate', (e) => {
  e.waitUntil(
    caches.keys()
      .then(keys => Promise.all(keys.filter(k => k !== CACHE).map(k => caches.delete(k))))
      .then(() => self.clients.claim())
  );
});

// COOP/COEP はサーバーが付与するので、キャッシュから返す際も保持される。
// ネットワーク優先、失敗時にキャッシュフォールバック。
self.addEventListener('fetch', (e) => {
  if (e.request.method !== 'GET') return;
  e.respondWith(
    fetch(e.request)
      .then(res => {
        if (res.ok) {
          const clone = res.clone();
          caches.open(CACHE).then(c => c.put(e.request, clone));
        }
        return res;
      })
      .catch(() => caches.match(e.request))
  );
});
