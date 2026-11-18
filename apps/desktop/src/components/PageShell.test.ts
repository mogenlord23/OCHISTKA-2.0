import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import fs from 'node:fs';
import path from 'node:path';
import PageShell from './PageShell.svelte';

afterEach(cleanup);

const snippet = (html: string) => createRawSnippet(() => ({ render: () => html }));

describe('PageShell', () => {
  it('renders title, lead, tabs and body in this order', () => {
    const { container } = render(PageShell, {
      props: {
        title: 'Заголовок',
        lead: 'Описание',
        tabs: snippet('<nav class="t">вкладки</nav>'),
        children: snippet('<p class="b">тело</p>'),
      },
    });
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Заголовок');
    const main = container.querySelector('main')!;
    const order = Array.from(main.querySelectorAll('h1, .lead, .t, .page-body'));
    expect(
      order.map((el) =>
        el.matches('h1')
          ? 'H1'
          : (['lead', 't', 'page-body'].find((c) => el.classList.contains(c)) ?? '?'),
      ),
    ).toEqual(['H1', 'lead', 't', 'page-body']);
    expect(container.querySelector('.page-body .b')?.textContent).toBe('тело');
  });

  it('works without tabs and without lead', () => {
    const { container } = render(PageShell, {
      props: { title: 'A', children: snippet('<p>x</p>') },
    });
    expect(container.querySelector('.lead')).toBeNull();
    expect(container.querySelector('.page-body')).toBeTruthy();
  });

  it('keeps one fixed layout: gutters, centred container and offset on the 8 px grid', () => {
    const css = fs.readFileSync(path.join(__dirname, 'PageShell.svelte'), 'utf8');
    expect(css).toMatch(/max-width:\s*1480px/);
    expect(css).toMatch(/margin-inline:\s*auto/);
    const style = css.slice(css.indexOf('<style>'));
    const values = [...style.matchAll(/(?:margin|padding|gap)[^:]*:\s*([^;]+);/g)]
      .flatMap((m) => m[1].match(/-?\d+(?:\.\d+)?px/g) ?? [])
      .map((v) => parseFloat(v));
    for (const v of values) expect(v % 8).toBe(0);
  });
});
