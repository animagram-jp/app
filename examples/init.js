const worker = new Worker('./worker.js', { type: 'module' });
let _reqId = 0;
const _pending = new Map();

worker.addEventListener('message', (e) => {
  const { id, type, payload } = e.data;
  if (type === 'execute') { payload.forEach(execute); return; }
  const resolve = _pending.get(id);
  if (resolve) { _pending.delete(id); resolve({ type, payload }); }
});

worker.addEventListener('error', (e) => {
  console.warn('worker error', e.message);
});

function call(type, payload = {}) {
  return new Promise((resolve) => {
    const id = _reqId++;
    _pending.set(id, resolve);
    worker.postMessage({ id, type, payload });
  });
}

const EVENT_TYPE = {
  click: 0b01,
  pointerdown: 0b10,
  submit: 0b11,
};

function dispatch(e) {
  const payload = {
    event_type: EVENT_TYPE[e.type] ?? 0,
    target_id: e.target.id,
    ctrl: e.ctrlKey ? 1 : 0,
    meta: e.metaKey ? 1 : 0,
  };
  if (e.type === 'submit') {
    e.preventDefault();
    payload.fields = Object.fromEntries(new FormData(e.target));
  }
  worker.postMessage({ type: 'event', payload });
}

function execute({ op, id, attr = '', value = '' }) {
  const el = document.getElementById(id);
  if (!el) return;
  switch (op) {
    case 0b01: // setAttribute / removeAttribute
      if (value === '') el.removeAttribute(attr);
      else el.setAttribute(attr, value);
      break;
    case 0b10: // setText
      el.textContent = value;
      break;
  }
}

function bind() {
  document.addEventListener('click', dispatch);
  document.getElementById('chat-form')?.addEventListener('submit', dispatch);

  // textarea に / を入力したら即セレクタを開く
  document.getElementById('chat-input')?.addEventListener('input', (e) => {
    if (e.target.value === '/') {
      e.target.value = '';
      worker.postMessage({
        type: 'event',
        payload: { event_type: 0b11, target_id: 'chat-form', fields: { text: '/' } },
      });
    }
  });

  // Esc でセレクタを閉じる
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      worker.postMessage({
        type: 'event',
        payload: { event_type: 0b01, target_id: 'selector-overlay' },
      });
    }
  });
}

worker.postMessage({ type: 'init' });
worker.addEventListener('message', (e) => {
  if (e.data.type === 'ready') bind();
}, { once: true });
