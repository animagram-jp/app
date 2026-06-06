const modal = document.getElementById('modal');
const openBtn = document.getElementById('header_button-3');

openBtn.addEventListener('click', () => {
  modal.showModal();
});

modal.addEventListener('click', (e) => {
  if (e.target === modal) {
    modal.close();
  }
});
