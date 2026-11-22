import { expect, it } from 'vitest';
import { price } from './price';

it('shows the amount', () => expect(price(5)).toBe('5 THB'));
