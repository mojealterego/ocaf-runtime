// Cache only public static editor assets, never API responses or credentials.
// CacheStorage is origin-wide, so isolate each service-worker registration scope.
const PREFIX = 'ocaf-builder-web:' + encodeURIComponent(self.registration.scope) + ':';
const CACHE = PREFIX + 'v14';
const ASSETS = ['./', './index.html', './style.css', './app.mjs', './preview-gate.mjs', './editor.mjs', './strict-json.mjs', './cloud.mjs', './i18n.mjs', './credentials.mjs', './models.mjs', './integrations-ui.mjs', './runner-client.mjs', './agent-catalog.mjs', './agent-catalog-ui.mjs', './agent-catalog.json', './manifest.webmanifest'];
self.addEventListener('install', event => {
  const requests = ASSETS.map(path => new Request(new URL(path, self.registration.scope), {
    cache: 'reload', credentials: 'omit', redirect: 'error'
  }));
  event.waitUntil(caches.open(CACHE).then(cache => cache.addAll(requests)).then(() => self.skipWaiting()));
});
self.addEventListener('activate', event => {
  event.waitUntil(caches.keys().then(keys => Promise.all(keys.filter(key => key.startsWith(PREFIX) && /^v[0-9]+$/.test(key.slice(PREFIX.length)) && key !== CACHE).map(key => caches.delete(key)))).then(() => self.clients.claim()));
});
self.addEventListener('fetch', event => {
  const url = new URL(event.request.url);
  if (event.request.method !== 'GET' || event.request.headers?.has('Authorization') || url.origin !== self.location.origin || !ASSETS.some(path => new URL(path, self.registration.scope).href === url.href)) return;
  event.respondWith(caches.open(CACHE).then(cache => cache.match(event.request)).then(hit => hit || fetch(event.request)));
});
