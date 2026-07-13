const params = new URLSearchParams(location.search);
if (params.has("eruda")) {
    const s = document.createElement("script");
    s.src = "https://cdn.jsdelivr.net/npm/eruda";
    s.onload = () => eruda.init();
    document.body.appendChild(s);
}

let worker = start();

function start() {
  const w = new Worker("./worker.js", { type: "module" });

  w.addEventListener("message", (e) => {
    const { type, payload } = e.data;
    if (type === "execute") { payload.forEach(execute); }
    if (type === "error")   { worker.terminate(); worker = start(); }
  });

  w.addEventListener("error", (e) => {
    console.error("[worker] restart:", e.message);
    worker.terminate();
    worker = start();
  });

  w.postMessage({
    type: "init",
    payload: {
      pointer_coarse:  window.matchMedia("(pointer: coarse)").matches,
      viewport_width:  window.innerWidth,
      viewport_height: window.innerHeight,
    },
  });

  let bound = false;
  w.addEventListener("message", function onReady(e) {
    if (e.data.type !== "ready" || bound) return;
    bound = true;
    w.removeEventListener("message", onReady);
    bind();
  });

  return w;
}

// ============================================================
// receive and excute commands
// ============================================================

// Command: op別のフィールドを持つタグ付きオブジェクト。operation番号(u8)はjs_client.rsのCommand enumと対応。
function execute(cmd) {
  const el = document.getElementById(cmd.id);
  if (!el) return;
  switch (cmd.operation) {
    case 1:  el.textContent = cmd.value ?? ""; break;
    case 2:  el.value = cmd.value ?? ""; break;
    case 3:  el.setAttribute(cmd.attribute, cmd.value ?? ""); break;
    case 4:  el.removeAttribute(cmd.attribute); break;
    case 5:  el.classList.add(cmd.value); break;
    case 6:  el.classList.remove(cmd.value); break;
    case 7:  el.style.width = cmd.px + "px"; break;
    case 8:  el.style.height = cmd.px + "px"; break;
    case 9:  el.style.zIndex = cmd.z; break;
    case 10: el.style.background = cmd.value; break;
    case 11: el.style.translate = `${cmd.x}px ${cmd.y}px`; break;
    case 12: el.style.cursor = cmd.value ?? ""; break;
    case 13: el.showModal(); break;
    case 14: el.close(); break;
    case 15: el.focus(); break;
    case 16: jsFn[cmd.name]?.(el); break;
  }
}

const jsFn = {
  show: (el) => {
    el.classList.remove("hidden");
    requestAnimationFrame(() => requestAnimationFrame(() => {
      el.classList.add("show");
      setTimeout(() => {
        el.classList.replace("show", "hide");
        el.addEventListener("transitionend", () => el.classList.remove("hide"), { once: true });
      }, 3000);
    }));
  },
  hide: (el) => {
    el.classList.replace("show", "hide");
    el.addEventListener("transitionend", () => el.classList.remove("hide"), { once: true });
  },
};

// ============================================================
// send event
// ============================================================

const ROOTS = ["header", "main", "modal", "form", "output", "section"]
  .map(id => document.getElementById(id));

function send(e) {
  if (!ROOTS.some(r => r && r.contains(e.target))) return;
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

  let resizeTimer;
  window.addEventListener("resize", () => {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => {
      worker.postMessage({ type: "event", payload: {
        event_type: "resize",
        target_id:  "",
        key:        "",
        value:      "",
        x:          window.innerWidth,
        y:          window.innerHeight,
        time:       performance.now(),
      }});
    }, 100);
  });

  window.addEventListener("pagehide", (e) => {
    if (!e.persisted) worker.postMessage({ type: "close" });
  });
}
