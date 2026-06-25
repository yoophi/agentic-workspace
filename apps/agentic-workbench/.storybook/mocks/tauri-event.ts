type Listener = (event: { payload: unknown }) => void;

const listeners = new Map<string, Set<Listener>>();

export async function listen(eventName: string, callback: Listener) {
  const eventListeners = listeners.get(eventName) ?? new Set<Listener>();
  eventListeners.add(callback);
  listeners.set(eventName, eventListeners);
  return () => {
    eventListeners.delete(callback);
  };
}

export function emitMockEvent(eventName: string, payload: unknown) {
  listeners.get(eventName)?.forEach((callback) => callback({ payload }));
}
