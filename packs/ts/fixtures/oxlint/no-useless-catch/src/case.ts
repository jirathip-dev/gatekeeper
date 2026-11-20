export function f(): unknown {
  try {
    return JSON.parse("x");
  } catch (e) {
    throw e;
  }
}
