self.addEventListener("message", async (e) => {
    const { type, payload } = e.data;
    if (type !== "init") return;

    const { default: init, App, arena_pointer, initialize, run_loop } =
        await import("./app/app.js");
    await init({ memory: payload.memory });

    initialize();
    const base = arena_pointer();
    self.postMessage({ type: "ready", base });

    await App.init(
        payload.pointer_coarse,
        payload.viewport_width,
        payload.viewport_height,
    );
    run_loop();
});

self.addEventListener("error", (e) => {
    self.postMessage({ type: "error", message: e.message });
});
