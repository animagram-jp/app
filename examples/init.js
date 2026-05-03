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
    case 0b1000: // showModal
      el.showModal();
      break;
    case 0b1001: // closeModal
      el.close();
      break;
  }
}

function bind() {
  document.getElementById('chat_form')?.addEventListener('submit', (e) => {
    e.preventDefault();
    dispatch({
      event_type: 0b010,
      target_id: 'chat_form',
      fields: Object.fromEntries(new FormData(e.target)),
    });
  });

  document.getElementById('chat_input')?.addEventListener('input', (e) => {
    dispatch({ event_type: 0b011, target_id: 'chat_input', value: e.target.value });
  });

  document.getElementById('char_edit_form')?.addEventListener('submit', (e) => {
    e.preventDefault();
    const dialog = document.getElementById('char_edit');
    const inputs = dialog.querySelectorAll('input[name]');
    const fields = Object.fromEntries(
      [...inputs].filter(i => i.value !== '').map(i => [i.name, i.value])
    );
    dispatch({ event_type: 0b010, target_id: 'char_edit_form', fields });
  });

  document.addEventListener('keydown', (e) => {
    if (['ArrowUp', 'ArrowDown', 'Enter', 'Escape'].includes(e.key)) {
      e.preventDefault();
      dispatch({ event_type: 0b100, target_id: e.target.id, key: e.key });
    }
  });

  document.getElementById('char_edit_open')?.addEventListener('click', (e) => {
    e.stopPropagation();
    dispatch({ event_type: 0b001, target_id: 'char_edit_open' });
  });

  document.addEventListener('click', (e) => {
    const el = e.target.closest('[id]');
    if (!el || el.id === 'char_edit_open') return;
    dispatch({ event_type: 0b001, target_id: el.id });
  });

  document.addEventListener('focusin', (e) => {
    const id = e.target.id;
    if (id.startsWith('roll_') || id.startsWith('char_roll_') || id.startsWith('skill_roll_')) {
      dispatch({ event_type: 0b110, target_id: id });
    }
  });
}

worker.postMessage({ type: 'init' });
worker.addEventListener('message', (e) => {
  if (e.data.type === 'ready') bind();
}, { once: true });
