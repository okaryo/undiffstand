import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import AiAnswer from './AiAnswer.svelte';

describe('AiAnswer', () => {
  it('shows evidence for a diff explanation', () => {
    render(AiAnswer, {
      explanation: {
        summary: 'The change prepares review context.',
        inferredIntent: 'Make review output traceable to evidence.',
        risk: 'medium',
        concerns: ['The new asynchronous step may fail.'],
        references: [{ path: 'src/lib/review.ts', startLine: 12, endLine: 18 }],
        caveats: ['The intent is inferred from the diff.']
      }
    });

    expect(screen.getByText('The change prepares review context.')).toBeInTheDocument();
    expect(screen.getByText('src/lib/review.ts')).toBeInTheDocument();
    expect(screen.getByText('L12–18')).toBeInTheDocument();
  });
});
