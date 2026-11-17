/** One-use validation correlation. This is not an authentication or signing boundary. */
export class PreviewGate {
  #sequence = 0;
  #pending = null;

  begin(transport = 'native') {
    if (transport !== 'native' && transport !== 'cloud') throw new Error('validation: invalid transport');
    if (this.#sequence >= Number.MAX_SAFE_INTEGER) throw new Error('validation: request counter exhausted');
    const id = String(++this.#sequence);
    this.#pending = {id, transport};
    return id;
  }

  invalidate() { this.#pending = null; }

  take(id, transport = 'native') {
    if (typeof id !== 'string' || this.#pending === null ||
        this.#pending.id !== id || this.#pending.transport !== transport) return false;
    this.invalidate();
    return true;
  }
}
