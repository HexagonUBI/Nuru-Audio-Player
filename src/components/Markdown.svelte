<script lang="ts">
  import { parseMarkdown, type Inline } from '$lib/markdown';
  import { openUrl } from '$lib/bridge';

  let { source }: { source: string } = $props();

  const blocks = $derived(parseMarkdown(source));

  function open(href: string) {
    void openUrl(href);
  }
</script>

{#snippet inline(parts: Inline[])}
  {#each parts as part, i (i)}
    {#if part.kind === 'strong'}
      <strong>{part.text}</strong>
    {:else if part.kind === 'em'}
      <em>{part.text}</em>
    {:else if part.kind === 'code'}
      <code>{part.text}</code>
    {:else if part.kind === 'link'}
      <button class="link" onclick={() => open(part.href)}>{part.text}</button>
    {:else}
      {part.text}
    {/if}
  {/each}
{/snippet}

<div class="md">
  {#each blocks as block, i (i)}
    {#if block.kind === 'heading'}
      {#if block.level === 1}
        <h3>{@render inline(block.content)}</h3>
      {:else if block.level === 2}
        <h4>{@render inline(block.content)}</h4>
      {:else}
        <h5>{@render inline(block.content)}</h5>
      {/if}
    {:else if block.kind === 'paragraph'}
      <p>{@render inline(block.content)}</p>
    {:else if block.kind === 'list'}
      {#if block.ordered}
        <ol>
          {#each block.items as item, j (j)}
            <li>{@render inline(item)}</li>
          {/each}
        </ol>
      {:else}
        <ul>
          {#each block.items as item, j (j)}
            <li>{@render inline(item)}</li>
          {/each}
        </ul>
      {/if}
    {:else if block.kind === 'code'}
      <pre>{block.text}</pre>
    {:else}
      <hr />
    {/if}
  {/each}
</div>

<style>
  .md {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    font: var(--t-body);
    color: var(--ink-80);
    line-height: 1.65;
  }

  h3,
  h4,
  h5 {
    font-family: var(--font-display);
    font-weight: 600;
    color: var(--ink);
    letter-spacing: -0.01em;
  }
  h3 {
    font-size: 18px;
  }
  h4 {
    font-size: 15px;
  }
  h5 {
    font-size: 13px;
    color: var(--ink-60);
  }

  ul,
  ol {
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding-left: 20px;
  }
  ul {
    list-style: disc;
  }
  ol {
    list-style: decimal;
  }
  li {
    display: list-item;
  }
  li::marker {
    color: var(--ink-25);
  }

  strong {
    color: var(--ink);
    font-weight: 600;
  }

  code {
    padding: 1px 5px;
    border-radius: var(--r-xs);
    background: rgba(255, 255, 255, 0.08);
    font-family: var(--font-ui);
    font-size: 0.92em;
    color: var(--ink);
  }

  pre {
    padding: var(--sp-3);
    border-radius: var(--r-sm);
    background: rgba(0, 0, 0, 0.3);
    font-size: 12px;
    white-space: pre-wrap;
    color: var(--ink-60);
    overflow-x: auto;
  }

  hr {
    border: none;
    border-top: 1px solid var(--line);
    margin: var(--sp-1) 0;
  }

  .link {
    color: var(--nuru);
    text-decoration: underline;
    text-underline-offset: 2px;
    font: inherit;
    padding: 0;
  }
  .link:hover {
    color: var(--nuru-bright);
  }
</style>
