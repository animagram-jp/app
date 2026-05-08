let app;

self.addEventListener("message", async (e) => {
  const { type, payload } = e.data;

  if (type === "init") {
    const { default: init, App } = await import("./dice-engine/app.js");
    await init();

    app = await App.init(payload.screen_width, payload.pointer_coarse);
    self.postMessage({ type: "ready" });
    const init_cmds = Array.from(app.flush() ?? []);
    if (init_cmds.length) self.postMessage({ type: "execute", payload: init_cmds });
    return;
  }

  if (!app || type !== "event") return;

  app.event(payload);
  const cmds = Array.from(app.flush() ?? []);
  if (cmds.length) self.postMessage({ type: "execute", payload: cmds });
});
