const worker = new Worker("./worker.js", { type: "module" });

// ── worker → main: DOM操作 ───────────────────────────────────────
worker.addEventListener("message", (e) => {
  const { type, payload } = e.data;
  if (type === "execute") { payload.forEach(execute); }
});

worker.addEventListener("error", (e) => {
  console.error("[worker]", e.message);
});

// ── DOM操作 ──────────────────────────────────────────────────────
// DomCmd: { operation: u8, id: string, attribute?: string, value?: string }
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

// ── main → worker: イベント送信 ──────────────────────────────────
function dispatch(payload) {
  worker.postMessage({ type: "event", payload });
}

function bind() {
  // click / backdrop close (modal, drawer外クリックを含む)
  document.addEventListener("click", (e) => {
    const el = e.target.closest("[id]");
    if (!el) return;
    dispatch({ event_type: "click", target_id: el.id });
  });

  // keydown: Appが意味を持つキーのみ転送
  document.addEventListener("keydown", (e) => {
    const keys = ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Enter", "Escape", "Tab"];
    if (!keys.includes(e.key)) return;
    e.preventDefault();
    dispatch({ event_type: "keydown", target_id: e.target.id ?? "", key: e.key });
  });

  // textarea input: "/" トリガー検知用
  document.getElementById("main_div_section-3_textarea")?.addEventListener("input", (e) => {
    dispatch({ event_type: "input", target_id: e.target.id, value: e.target.value });
  });

  // modal内 number input: 値をそのままRust側に渡す
  document.getElementById("modal")?.addEventListener("input", (e) => {
    const el = e.target;
    if (el.tagName !== "INPUT" || el.type !== "number") return;
    dispatch({
      event_type: "input",
      target_id: el.id,
      value: String(isNaN(el.valueAsNumber) ? 0 : el.valueAsNumber),
    });
  });
}

// ── 初期化 ───────────────────────────────────────────────────────
worker.postMessage({ type: "init" });
worker.addEventListener("message", (e) => {
  if (e.data.type === "ready") bind();
}, { once: true });
