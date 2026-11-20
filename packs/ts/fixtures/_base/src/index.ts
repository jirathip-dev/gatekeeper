import { double } from "./math";

export function quadruple(n: number): number {
  return double(double(n));
}
