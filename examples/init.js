const worker = new Worker("./worker.js", { type: "module" });

// ============================================================
// receive canvas commands and excute
// ============================================================

worker.addEventListener("message", (e) => {
  const { type, payload } = e.data;
  if (type === "execute") { payload.forEach(execute); }
});

worker.addEventListener("error", (e) => {
  console.error("[worker]", e.message);
});

// CanvasCmd: { operation: u8, id: string, attribute?: string, value?: string }
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

  document.addEventListener("keydown", (e) => {
    const keys = ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Enter", "Escape", "Tab"];
    if (!keys.includes(e.key)) return;
    e.preventDefault();
    dispatch({ event_type: "keydown", target_id: e.target.id ?? "", key: e.key });
  });

  // character select: キャラ切り替え
  document.getElementById("main_div_section-1_section-1_select")?.addEventListener("change", (e) => {
    dispatch({ event_type: "change", target_id: e.target.id, value: e.target.value });
  });

  // textarea input: "/" トリガー検知用
  document.getElementById("main_div_section-3_textarea")?.addEventListener("input", (e) => {
    dispatch({ event_type: "input", target_id: e.target.id, value: e.target.value });
  });

  // modal内 select: spec選択をRust側に渡す
  document.getElementById("modal")?.addEventListener("change", (e) => {
    const el = e.target;
    if (el.tagName !== "SELECT") return;
    dispatch({ event_type: "change", target_id: el.id, value: el.value });
  });

  // modal内 spec input: focusout時にRust側へ通知（空かどうかはwasm側で判断）
  document.getElementById("modal")?.addEventListener("focusout", (e) => {
    const el = e.target;
    if (el.tagName !== "INPUT" || el.type !== "text") return;
    if (!el.id.endsWith("_td-1_input")) return;
    dispatch({ event_type: "blur", target_id: el.id, value: el.value });
  });

  // modal内 input: number と text を Rust 側に渡す
  document.getElementById("modal")?.addEventListener("input", (e) => {
    const el = e.target;
    if (el.tagName !== "INPUT") return;
    if (el.type === "number") {
      dispatch({
        event_type: "input",
        target_id: el.id,
        value: String(isNaN(el.valueAsNumber) ? 0 : el.valueAsNumber),
      });
    } else if (el.type === "text") {
      dispatch({ event_type: "input", target_id: el.id, value: el.value });
    }
  });
}

// ── 初期化 ───────────────────────────────────────────────────────
worker.postMessage({
  type: "init",
  payload: {
    screen_width:   screen.width,
    pointer_coarse: window.matchMedia("(pointer: coarse)").matches,
  },
});
worker.addEventListener("message", (e) => {
  if (e.data.type === "ready") bind();
}, { once: true });
