import { expect, it } from 'vitest';
import { button } from './button';

it('keeps its label', () => expect(button('ok')).toBe('ok'));
