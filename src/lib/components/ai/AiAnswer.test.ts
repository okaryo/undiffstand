import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import AiAnswer from './AiAnswer.svelte';

describe('AiAnswer', () => {
  it('opens a cited source reference', async () => {
    const onReference = vi.fn();
    const reference = { path: 'src/lib/review.ts', startLine: 12, endLine: 18 };
    render(AiAnswer, {
      answer: {
        answer: 'The function prepares review context.',
        references: [reference],
        caveats: ['The intent is inferred from the selected code.']
      },
      onReference
    });

    await fireEvent.click(screen.getByRole('button', { name: /src\/lib\/review\.ts/i }));
    expect(onReference).toHaveBeenCalledWith(reference);
  });
});
