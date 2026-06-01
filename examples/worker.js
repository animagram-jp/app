let app;

self.addEventListener("message", async (e) => {
  const { type, payload } = e.data;

  if (type === "init") {
    const { default: init, App } = await import("./app/app.js");
    await init();

    app = await App.init(payload.screen_width, payload.pointer_coarse);
    self.postMessage({ type: "ready" });
    const init_cmds = app.event({});
    if (init_cmds?.length) self.postMessage({ type: "execute", payload: Array.from(init_cmds) });
    return;
  }

  if (!app || type !== "event") return;

  const cmds = app.handle(payload);
  if (cmds?.length) self.postMessage({ type: "execute", payload: Array.from(cmds) });
});

self.addEventListener("error", (e) => {
  self.postMessage({ type: "error", message: e.message });
});
