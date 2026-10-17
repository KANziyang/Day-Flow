export function movedBeyondClick(start: { x: number; y: number }, next: { x: number; y: number }) {
  return Math.hypot(next.x - start.x, next.y - start.y) >= 5;
}
