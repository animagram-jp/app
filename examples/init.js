let worker = start();

function start() {
  const w = new Worker("./worker.js", { type: "module" });

  w.addEventListener("message", (e) => {
    const { type, payload } = e.data;
    if (type === "execute") { payload.forEach(execute); }
    if (type === "error")   { restart(e.data.message); }
  });

  w.addEventListener("error", (e) => {
    console.error("[worker] restart:", e.message);
    worker.terminate();
    worker = start();
  });

  w.postMessage({
    type: "init",
    payload: {
      screen_width:   screen.width,
      pointer_coarse: window.matchMedia("(pointer: coarse)").matches,
    },
  });

  w.addEventListener("message", (e) => {
    if (e.data.type === "ready") bind();
  }, { once: true });

  return w;
}

// ============================================================
// receive and excute canvas commands
// ============================================================

// Command: { operation: u8, id: string, attribute?: string, value?: string }
function execute({ operation, id, attribute, value }) {
  const el = document.getElementById(id);
  if (!el) return;
  switch (operation) {
    case 1: el.textContent = value ?? ""; break;
    case 2: el.value = value ?? ""; break;
    case 3: el.toggleAttribute(attribute, value); break;
    case 4: el.classList.add(value); break;
    case 5: el.classList.remove(value); break;
    case 6: el.focus(); break;
    case 7: el.showModal(); break;
    case 8: el.close(); break;
    case 9: applyClass(el, value); break;
    case 10: el.innerHTML = value ?? ""; break;
  }
}

function applyClass(el, value) {
  if (value === "show") {
    el.classList.remove("hidden");
    requestAnimationFrame(() => requestAnimationFrame(() => {
      el.classList.add("show");
      setTimeout(() => {
        el.classList.replace("show", "hide");
        el.addEventListener("transitionend", () => el.classList.remove("hide"), { once: true });
      }, 3000);
    }));
  } else if (value === "hide") {
    el.classList.replace("show", "hide");
    el.addEventListener("transitionend", () => el.classList.remove("hide"), { once: true });
  }
}

// ============================================================
// send event
// ============================================================

function send(e) {
  worker.postMessage({ type: "event", payload: {
    event_type: e.type,
    target_id:  e.target.id ?? "",
    key:        e.key ?? "",
    value:      e.target.value ?? "",
    x:          e.clientX ?? 0,
    y:          e.clientY ?? 0,
    time:       e.timeStamp ?? 0,
  }});
}

function bind() {
  const EVENTS = ["click", "keydown", "input", "change", "submit", "focusout",
                  "pointerdown", "pointerup", "pointermove", "pointercancel"];
  for (const type of EVENTS) {
    document.addEventListener(type, send);
  }
}
