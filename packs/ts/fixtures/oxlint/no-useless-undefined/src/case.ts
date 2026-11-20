export function f(g: (n?: number) => void): void {
  g(undefined);
}
