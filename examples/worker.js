let app;

self.addEventListener('message', async (e) => {
  const { type, payload } = e.data;

  if (type === 'init') {
    const { default: init, App } = await import('./dice-engine/app.js');
    await init();

    app = await App.init();
    const cmds = app.event({});
    self.postMessage({ type: 'ready' });
    if (cmds?.length) self.postMessage({ type: 'execute', payload: cmds });
    return;
  }

  if (!app) return;

  const cmds = app[type](payload);
  if (cmds?.length) self.postMessage({ type: 'execute', payload: cmds });
});
