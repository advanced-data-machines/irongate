// Read-only status polling; actions stay ordinary one-use POST forms.
async function observe() {
  try {
    const reply = await fetch('/status', {cache: 'no-store'});
    if (!reply.ok) throw new Error('status unavailable');
    const state = await reply.json();
    const badge = document.querySelector('.badge');
    if (badge) badge.textContent = state.allowed ? 'Connected' : 'Access paused';
    if (String(state.allowed) !== document.body.dataset.allowed) {
      location.reload();
      return;
    }
    const records = document.querySelector('#records');
    if (records) records.textContent = `Local records: ${state.records.retained} retained, ${state.records.durable} saved.`;
  } catch {
    const badge = document.querySelector('.badge');
    if (badge) badge.textContent = 'Status unavailable — refresh to retry';
  }
  setTimeout(observe, 1000);
}
setTimeout(observe, 1000);
