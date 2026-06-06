const modal = document.getElementById('modal');
const openBtn = document.getElementById('header_button-3');

openBtn.addEventListener('click', () => {
  modal.showModal();
});

modal.addEventListener('click', (e) => {
  const rect = modal.getBoundingClientRect();
  if (e.clientX < rect.left || e.clientX > rect.right || e.clientY < rect.top || e.clientY > rect.bottom) {
    modal.close();
  }
});
