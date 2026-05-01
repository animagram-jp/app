const worker = new Worker('./worker.js', { type: 'module' });

worker.addEventListener('message', (e) => {
  const { type, payload } = e.data;
  if (type === 'execute') { payload.forEach(execute); return; }
});

worker.addEventListener('error', (e) => {
  console.warn('worker error', e.message);
});

function dispatch(payload) {
  worker.postMessage({ type: 'event', payload });
}

function execute({ op, id, attr = '', value = '' }) {
  const el = document.getElementById(id);
  if (!el) return;
  switch (op) {
    case 0b001: // setAttribute / removeAttribute
      if (attr === 'value') { el.value = value; }
      else if (value === '') el.removeAttribute(attr);
      else el.setAttribute(attr, value);
      break;
    case 0b010: // setText
      el.textContent = value;
      break;
    case 0b100: // focus
      el.focus();
      break;
  }
}

function bind() {
  document.getElementById('chat-form')?.addEventListener('submit', (e) => {
    e.preventDefault();
    dispatch({
      event_type: 0b010,
      target_id: 'chat-form',
      fields: Object.fromEntries(new FormData(e.target)),
    });
  });

  document.getElementById('chat-input')?.addEventListener('input', (e) => {
    dispatch({ event_type: 0b011, target_id: 'chat-input', value: e.target.value });
  });

  document.getElementById('char-stats')?.addEventListener('change', (e) => {
    dispatch({ event_type: 0b101, target_id: e.target.id, value: e.target.value });
  });

  document.addEventListener('keydown', (e) => {
    if (['ArrowUp', 'ArrowDown', 'Enter', 'Escape'].includes(e.key)) {
      e.preventDefault();
      dispatch({ event_type: 0b100, target_id: e.target.id, key: e.key });
    }
  });

  document.addEventListener('click', (e) => {
    dispatch({ event_type: 0b001, target_id: e.target.id });
  });

  document.addEventListener('focusin', (e) => {
    const id = e.target.id;
    if (id.startsWith('roll-') || id.startsWith('charroll-')) {
      dispatch({ event_type: 0b110, target_id: id });
    }
  });
}

worker.postMessage({ type: 'init' });
worker.addEventListener('message', (e) => {
  if (e.data.type === 'ready') bind();
}, { once: true });
