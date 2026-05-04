let app;

self.addEventListener('message', async (e) => {
  const { type, payload } = e.data;

  if (type === 'init') {
    const { default: init, App } = await import('./dice-engine/app.js');
    await init();

    app = await App.init();
    self.postMessage({ type: 'ready' });
    return;
  }

  if (!app) return;

  const dom_cmds = Array.from(app[type](payload) ?? []);
  if (dom_cmds.length) self.postMessage({ type: 'execute', payload: dom_cmds });
});
