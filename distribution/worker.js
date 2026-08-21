let app;

self.addEventListener("message", async (e) => {
    const { type, payload } = e.data;

    if (type === "init") {
        const { default: init, App } = await import("./app/app.js");
        await init();

    app = await App.init(payload.pointer_coarse, payload.viewport_width ?? 0, payload.viewport_height ?? 0);
    self.postMessage({ type: "ready" });
    const init_commands = app.process({});
    if (init_commands?.length) self.postMessage({ type: "execute", payload: Array.from(init_commands) });
        return;
    }

    if (!app) return;

    if (type === "close") { app.close(); self.close(); return; }

    if (type !== "event") return;

    const commands = app.process(payload);
    if (commands?.length) self.postMessage({ type: "execute", payload: Array.from(commands) });
});

self.addEventListener("error", (e) => {
    self.postMessage({ type: "error", message: e.message });
});
