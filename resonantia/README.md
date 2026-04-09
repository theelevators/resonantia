# Resonantia — Weaver UI

Four files. Drop them into your Svelte src directory and replace your current routes/views with `<Weaver />`.

## Files

```
types.ts          — shared TypeScript interfaces (mirrors your Rust DTOs exactly)
avec.ts           — AVEC color math, formatting helpers
CollapseCard.svelte — the card that surfaces from the terrain on collapse tap
OrbSystem.svelte  — the live AVEC icosahedral orb with attractor streams
Weaver.svelte     — the main surface: constellation, sessions, nodes, full chrome
```

## Integration

### 1. Install fonts

In your `app.html` or global CSS:

```html
<link
  href="https://fonts.googleapis.com/css2?family=Departure+Mono&family=Fraunces:ital,wght@0,300;0,400;1,300&display=swap"
  rel="stylesheet"
/>
```

Or self-host them via `@fontsource` if you prefer offline-first (recommended for Tauri):

```bash
npm install @fontsource/fraunces
```

Then in your global CSS:
```css
@import '@fontsource/fraunces/300.css';
@import '@fontsource/fraunces/300-italic.css';
```

Departure Mono: https://github.com/rektdeckard/departure-mono (free, self-host the woff2)

### 2. Replace your root route

```svelte
<!-- src/routes/+page.svelte or your App.svelte -->
<script>
  import Weaver from '$lib/Weaver.svelte';
</script>

<Weaver />
```

### 3. Tauri command bindings

`Weaver.svelte` calls these commands — they map directly to your existing `lib.rs`:

| Svelte call                          | Rust command       |
|--------------------------------------|--------------------|
| `invoke('get_graph', { limit, sessionId })` | `get_graph`  |
| `invoke('list_nodes', { limit, sessionId })` | `list_nodes` |

The `CollapseCard` receives a `NodeDto` fetched by `list_nodes` and displays
`user_avec`, `rho`, `kappa`, and `psi` directly — no `unwind_node` called,
no coaching language surfaced.

### 4. OrbSystem usage

Use `OrbSystem` anywhere you want the live orb rendered — it's a standalone canvas component:

```svelte
<script>
  import OrbSystem from '$lib/OrbSystem.svelte';
  import type { AvecState } from '$lib/types';

  const avec: AvecState = {
    stability: 0.95,
    friction:  0.19,
    logic:     0.81,
    autonomy:  0.99,
    psi:       2.94,
  };
</script>

<OrbSystem {avec} size={200} showStreams={true} />
```

## Design decisions

- **No page transitions** — the Weaver is one surface. Sessions zoom in-place.
- **No coaching language** — `unwind_node` result is intentionally not shown.
  The card shows the raw AVEC fingerprint, Ψ, ρ, κ. The user reads the terrain.
- **AVEC colors**: stability=#e8e8eb · friction=#e9943a · logic=#5b9bd5 · autonomy=#7aaa7a
- **Fonts**: Fraunces (display, italic, 300wt) + Departure Mono (mono UI)
- **Pan** — drag the canvas. Click a session wave to enter it.
  Click a collapse node to surface the card. Click empty space to surface back up.

## Next steps

- Wire `compose-btn` to your `store_context` flow
- Wire `calibrate` nav button to `calibrate_session`  
- Add the orb bloom descent animation when entering a collapse (zoom + canvas scale tween)
- Add `compression_avec` drift visualization to the card threads
