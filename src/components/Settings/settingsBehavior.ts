export function nextFocusIndex(count: number, current: number, backwards: boolean): number {
  if (count <= 0) return -1;
  if (backwards) return current <= 0 ? count - 1 : current - 1;
  return current < 0 || current >= count - 1 ? 0 : current + 1;
}

export async function persistWithRollback(
  persist: () => Promise<void>,
  rollback: () => void,
): Promise<void> {
  try {
    await persist();
  } catch (error) {
    rollback();
    throw error;
  }
}
