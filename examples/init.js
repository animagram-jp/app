const worker = new Worker("./worker.js", { type: "module" })

worker.addEventListener("message", (e) => {
  const { type, payload } = e.data;
  if (type === "execute") { payload.forEach(execute); return; }
})

worker.addEventListener("error", (e) => {
  execute(4, "output_article1", "warning");
  execute(1, "output_article1_span", "!");
  execute(1, "output_article1_p", e.message);
  execute(8, "output_article1", "show");
  // worker.terminate(); worker = new Worker("worker.js");  // 再起動する場合
});

const execute = (operation, element_id, attribute = '', value = '') => {
  const element = document.getElementById(element_id);
  if (!element) return;
  switch(operation) {
    case 1: element.textContent = value; break
    case 2: element.value = value; break
    case 3: element.toggleAttribute(attribute, value); break
    case 4: element.classList.add(value); break
    case 5: element.classList.remove(value); break
    case 6: element.focus(); break // preventScroll: trueも引数で可能なので、デフォの挙動のおかしい環境があれば検討
    case 7: element.openModal(); break
    case 8: element.close(); break
    case 9: applyClass(element, value); break
  }
}

function applyClass(element, value) {
  switch(value) {
    case "show": 
      element.classList.remove("hide");
      requestAnimationFrame(() => requestAnimationFrame(() => element.classList.add("show"))); break
    case "hide":
      element.classList.replace("show", "hide");
  }
}

function bind() {
  document.getElementById("form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    worker.postMessage({ 
      type: "event",
      event_type: "submit", 
      target_id: "form", 
      value: Object.fromEntries(new FormData(e.target)),
    })
  })
  document.getElementById("input")?.addEventListener("input", (e) => {
    worker.postMessage({ 
      type: "event",
      payload: {
        event_type: "input", 
        target_id: e.target.id, 
        value: e.target.value,
      }
    })
  })
  document.addEventListener("keydown", (e) => {
    if (["ArrowUp", "ArrowDown", "Enter", "Escape"].includes(e.key)) {
      e.preventDefault();
      worker.postMessage({ 
        type: "event",
        event_type: "keydown", 
        target_id: e.target.id, 
        value: e.key,
    })}
  })
  document.getElementById("main_header_button")?.addEventListener("click", (e) => {
    e.stopPropagation();
    dispatch({ event_type: e.type, target_id: "main_header_button" });
  });
  document.addEventListener("click", (e) => {
    const element = e.target.closest('[id]');
    if (!element || ["main_header_button"].includes(element.id)) return;
    worker.postMessage({ 
        type: "event",
        event_type: "click", 
        target_id: element.id, 
    })
  });
  document.addEventListener("focusin", (e) => {
    const id = e.target.id;
    if (id.contains("")) {
      worker.postMessage({ 
          type: "event",
          event_type: "focusin", 
          target_id: element.id, 
      })
    }
  });
}
worker.postMessage({ type: "init" });
worker.addEventListener("message", (e) => {
  if (e.data.type === "ready") bind();
}, { once: true });