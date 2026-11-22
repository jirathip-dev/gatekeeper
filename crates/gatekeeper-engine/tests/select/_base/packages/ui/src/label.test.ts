import { expect, it } from 'vitest';
import { label } from './label';

it('shouts', () => expect(label('ok')).toBe('OK'));
