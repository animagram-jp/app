let app;

self.addEventListener("message", async (e) => {
  const { type, payload } = e.data;

  if (type === "init") {
    const { default: init, App } = await import("./dice-engine/app.js");
    await init();

    const [a, init_cmds] = await App.init(payload.screen_width, payload.pointer_coarse);
    app = a;
    self.postMessage({ type: "ready" });
    if (init_cmds?.length) self.postMessage({ type: "execute", payload: Array.from(init_cmds) });
    return;
  }

  if (!app || type !== "event") return;

  const cmds = app.event(payload);
  if (cmds?.length) self.postMessage({ type: "execute", payload: Array.from(cmds) });
});

self.addEventListener("error", (e) => {
  self.postMessage({ type: "error", message: e.message });
});
