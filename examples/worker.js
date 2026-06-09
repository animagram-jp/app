let app;

self.addEventListener("message", async (e) => {
  const { type, payload } = e.data;

  if (type === "init") {
    const { default: init, App } = await import("./app/app.js");
    await init();

    app = await App.init(payload.screen_width, payload.pointer_coarse);
    self.postMessage({ type: "ready" });
    const init_commands = app.process({});
    if (init_commands?.length) self.postMessage({ type: "execute", payload: Array.from(init_commands) });
    return;
  }

  if (!app || type !== "event") return;

  const commands = app.process(payload);
  if (commands?.length) self.postMessage({ type: "execute", payload: Array.from(commands) });
});

self.addEventListener("error", (e) => {
  self.postMessage({ type: "error", message: e.message });
});
