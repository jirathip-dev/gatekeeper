export function f(a: boolean, b: boolean): number {
  let n = 0;
  if (a) {
    n = 1;
  } else {
    if (b) {
      n = 2;
    }
  }
  return n;
}
