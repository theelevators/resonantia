<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { avecColor, avecToRgb, shortLabel, AVEC_HEX, formatTimestamp } from './avec';
  import CollapseCard from './CollapseCard.svelte';
  import type {
    GraphResponse,
    GraphSessionDto,
    GraphNodeDto,
    NodeDto,
    CollapseCardData,
    Vec2,
  } from './types';

  const FONT_MONO    = "'Departure Mono', 'Courier New', monospace";
  const FONT_DISPLAY = "'Fraunces', Georgia, serif";

  // ── Canvas ──────────────────────────────────────────
  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let raf: number;
  let container: HTMLDivElement;

  function W() { return canvas?.width  ?? 800; }
  function H() { return canvas?.height ?? 600; }

  // ── Data ──────────────────────────────────────────────────
  let graph: GraphResponse | null = null;
  let loading = true;
  let error: string | null = null;

  const sessionPos: Record<string, Vec2> = {};
  const nodePos:    Record<string, Vec2> = {};

  // ── Camera ──────────────────────────────────────────────
  // Camera: the world point at screen-center, and the current scale
  let camX = 0, camY = 0, camScale = 1;
  let targetCamX = 0, targetCamY = 0, targetCamScale = 1;

  const WAVE_SCALE     = 3.5;
  const COLLAPSE_SCALE = 8;
  const LERP           = 0.09;

  // ── Level state machine ────────────────────────────────────────
  // 0 = constellation  1 = wave (session)  2 = collapse (node/moment)
  let level = 0;
  let selectedSession: GraphSessionDto | null = null;
  let selectedNode:    GraphNodeDto    | null = null;
  let cardData:    CollapseCardData | null = null;
  let cardVisible  = false;

  // ── Interaction ───────────────────────────────────────────
  let dragging    = false;
  let didDrag     = false;
  let dragStart:   Vec2 = { x: 0, y: 0 };
  let panCamStart: Vec2 = { x: 0, y: 0 };

  let t = 0;
  const STATIC_STARS = Array.from({ length: 80 }, () => ({
    x:     Math.random(),
    y:     Math.random(),
    s:     0.3 + Math.random() * 0.9,
    phase: Math.random() * Math.PI * 2,
  }));

  // ── Coordinate helpers ──────────────────────────────────────────
  function toScreen(wx: number, wy: number): Vec2 {
    return {
      x: (wx - camX) * camScale + W() / 2,
      y: (wy - camY) * camScale + H() / 2,
    };
  }

  function toWorld(sx: number, sy: number): Vec2 {
    return {
      x: (sx - W() / 2) / camScale + camX,
      y: (sy - H() / 2) / camScale + camY,
    };
  }

  function canvasXY(e: MouseEvent): Vec2 {
    const rect = canvas.getBoundingClientRect();
    return {
      x: (e.clientX - rect.left) * (canvas.width  / rect.width),
      y: (e.clientY - rect.top)  * (canvas.height / rect.height),
    };
  }

  // ── Data loading ─────────────────────────────────────────────
  async function loadGraph() {
    try {
      loading = true;
      error   = null;
      graph   = await invoke<GraphResponse>('get_graph', { limit: 200, sessionId: null });
      layoutConstellation();
      camX = targetCamX = W() / 2;
      camY = targetCamY = H() / 2;
      camScale = targetCamScale = 1;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function layoutConstellation() {
    if (!graph) return;
    const cx = W() / 2, cy = H() / 2;
    const n  = graph.sessions.length;
    const r  = Math.min(W(), H()) * 0.28;

    graph.sessions.forEach((s, i) => {
      const angle = (i / n) * Math.PI * 2 - Math.PI / 2;
      sessionPos[s.id] = {
        x: cx + Math.cos(angle) * r,
        y: cy + Math.sin(angle) * r * 0.72,
      };
    });

    graph.nodes.forEach((nd, i) => {
      const sp = sessionPos[nd.sessionId];
      if (!sp) return;
      const angle = (i * 2.399) % (Math.PI * 2);
      const dist  = 22 + (i % 5) * 9;
      nodePos[nd.id] = {
        x: sp.x + Math.cos(angle) * dist,
        y: sp.y + Math.sin(angle) * dist,
      };
    });
  }

  function sessionRadius(s: GraphSessionDto) { return 10 + s.nodeCount * 2.2; }
  function nodeRadius(n: GraphNodeDto)        { return 4  + n.psi * 1.6;      }

  // ── Navigation ────────────────────────────────────────────────
  function descendToWave(s: GraphSessionDto) {
    selectedSession = s;
    selectedNode    = null;
    closeCard();
    level = 1;
    const sp = sessionPos[s.id];
    if (sp) { targetCamX = sp.x; targetCamY = sp.y; targetCamScale = WAVE_SCALE; }
  }

  async function descendToCollapse(n: GraphNodeDto) {
    selectedNode = n;
    level        = 2;
    const np = nodePos[n.id];
    if (np) { targetCamX = np.x; targetCamY = np.y; targetCamScale = COLLAPSE_SCALE; }

    cardData = {
      node:            n,
      nodeDto:         null,
      relatedSessions: graph?.sessions.filter(s => s.id !== n.sessionId).slice(0, 4) ?? [],
    };
    setTimeout(() => { cardVisible = true; }, 650);

    try {
      const res = await invoke<{ nodes: NodeDto[] }>('list_nodes', {
        limit: 50, sessionId: n.sessionId,
      });
      const dto = res.nodes[0] ?? null;
      if (dto && cardData) cardData = { ...cardData, nodeDto: dto };
    } catch { /* card shows what it has */ }
  }

  function surfaceToWave() {
    selectedNode = null;
    closeCard();
    level = 1;
    if (selectedSession) {
      const sp = sessionPos[selectedSession.id];
      if (sp) { targetCamX = sp.x; targetCamY = sp.y; targetCamScale = WAVE_SCALE; }
    }
  }

  function surfaceToConstellation() {
    selectedSession = null;
    selectedNode    = null;
    closeCard();
    level = 0;
    targetCamX = W() / 2; targetCamY = H() / 2; targetCamScale = 1;
  }

  function closeCard() {
    cardVisible = false;
    setTimeout(() => { if (!cardVisible) cardData = null; }, 500);
  }

  function handleNavigate(e: CustomEvent<{ sessionId: string }>) {
    const target = graph?.sessions.find(s => s.id === e.detail.sessionId);
    if (target) descendToWave(target);
  }

  // ── Draw helpers ─────────────────────────────────────────────
  function drawStars() {
    const dim = level === 0 ? 1 : level === 1 ? 0.45 : 0.15;
    STATIC_STARS.forEach(s => {
      const a = (0.15 + Math.sin(t * 0.6 + s.phase) * 0.1) * dim;
      ctx.beginPath();
      ctx.arc(s.x * W(), s.y * H(), s.s, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(255,255,255,${a})`;
      ctx.fill();
    });
  }

  function drawEdges() {
    if (!graph || level > 0) return;
    graph.edges.forEach(e => {
      const sp = sessionPos[e.source]; const tp = sessionPos[e.target];
      if (!sp || !tp) return;
      const ss = toScreen(sp.x, sp.y); const ts = toScreen(tp.x, tp.y);
      const mx = (ss.x + ts.x) / 2 + Math.sin(e.id.length * 0.7) * 30;
      const my = (ss.y + ts.y) / 2 + Math.cos(e.id.length * 0.5) * 20;
      const grad = ctx.createLinearGradient(ss.x, ss.y, ts.x, ts.y);
      grad.addColorStop(0,    'rgba(255,255,255,0)');
      grad.addColorStop(0.35, 'rgba(255,255,255,0.06)');
      grad.addColorStop(0.5,  e.kind === 'resonance' ? 'rgba(255,255,255,0.1)' : 'rgba(255,255,255,0.04)');
      grad.addColorStop(0.65, 'rgba(255,255,255,0.06)');
      grad.addColorStop(1,    'rgba(255,255,255,0)');
      ctx.beginPath();
      ctx.moveTo(ss.x, ss.y);
      ctx.quadraticCurveTo(mx, my, ts.x, ts.y);
      ctx.strokeStyle = grad;
      ctx.lineWidth   = e.kind === 'resonance' ? 0.8 : 0.4;
      ctx.setLineDash(e.kind === 'temporal' ? [3, 6] : []);
      ctx.stroke();
      ctx.setLineDash([]);
    });
  }

  function drawSessions() {
    if (!graph) return;
    const baseAvec = { stability:0.7, friction:0.2, logic:0.8, autonomy:0.85, psi:2.55 };

    graph.sessions.forEach(s => {
      const sp      = sessionPos[s.id];
      if (!sp) return;
      const sc      = toScreen(sp.x, sp.y);
      const isFocus = selectedSession?.id === s.id;

      if (level > 0 && !isFocus) {
        ctx.beginPath();
        ctx.arc(sc.x, sc.y, 2, 0, Math.PI * 2);
        ctx.fillStyle = 'rgba(255,255,255,0.07)';
        ctx.fill();
        return;
      }

      const r     = sessionRadius(s);
      const pulse = isFocus && level === 1
        ? Math.sin(t * 3) * 2.5
        : Math.sin(t * 1.1 + sp.x * 0.008) * 1.2;

      ctx.beginPath();
      ctx.arc(sc.x, sc.y, r + pulse + 12, 0, Math.PI * 2);
      ctx.fillStyle = avecColor(baseAvec, 0.03);
      ctx.fill();

      ctx.beginPath();
      ctx.arc(sc.x, sc.y, r + pulse, 0, Math.PI * 2);
      ctx.fillStyle   = avecColor(baseAvec, isFocus ? 0.14 : 0.07);
      ctx.fill();
      ctx.strokeStyle = avecColor(baseAvec, isFocus ? 0.48 : 0.18);
      ctx.lineWidth   = isFocus ? 0.8 : 0.5;
      ctx.stroke();

      ctx.beginPath();
      ctx.arc(sc.x, sc.y, 2.5, 0, Math.PI * 2);
      ctx.fillStyle = avecColor(baseAvec, 0.9);
      ctx.fill();

      if (level === 0 && r > 18) {
        ctx.fillStyle = 'rgba(255,255,255,0.22)';
        ctx.font      = `10px ${FONT_MONO}`;
        ctx.textAlign = 'center';
        ctx.fillText(shortLabel(s.label, 3), sc.x, sc.y + r + 15);
      }
    });
  }

  function drawWaveBoundary() {
    if (level !== 1 || !selectedSession) return;
    const sp = sessionPos[selectedSession.id];
    if (!sp) return;
    const sc  = toScreen(sp.x, sp.y);
    const r   = sessionRadius(selectedSession);
    const rx  = (r + 30) * camScale;
    const ry  = (r + 30) * camScale * 0.72;
    const oda = 4 + Math.sin(t * 0.5);

    ctx.beginPath();
    ctx.ellipse(sc.x, sc.y, rx + 22, ry + 16, 0, 0, Math.PI * 2);
    ctx.strokeStyle = 'rgba(122,170,122,0.04)';
    ctx.lineWidth   = 12;
    ctx.setLineDash([]);
    ctx.stroke();

    ctx.beginPath();
    ctx.ellipse(sc.x, sc.y, rx, ry, 0, 0, Math.PI * 2);
    ctx.strokeStyle = 'rgba(122,170,122,0.2)';
    ctx.lineWidth   = 0.5;
    ctx.setLineDash([oda, oda * 1.6]);
    ctx.stroke();
    ctx.setLineDash([]);
  }

  function drawNodes() {
    if (!graph || level < 1 || !selectedSession) return;
    const av = { stability:0.75, friction:0.18, logic:0.85, autonomy:0.9, psi:2.68 };
    const sessionNodes = graph.nodes.filter(n => n.sessionId === selectedSession!.id);

    sessionNodes.forEach(n => {
      const np = nodePos[n.id];
      if (!np) return;
      const sc         = toScreen(np.x, np.y);
      const r          = nodeRadius(n);
      const isSelected = selectedNode?.id === n.id;

      if (level === 2 && !isSelected) {
        ctx.beginPath();
        ctx.arc(sc.x, sc.y, 2, 0, Math.PI * 2);
        ctx.fillStyle = 'rgba(255,255,255,0.05)';
        ctx.fill();
        return;
      }

      const pulse = Math.sin(t * 1.8 + np.x * 0.04);

      ctx.beginPath();
      ctx.arc(sc.x, sc.y, r + pulse + 7, 0, Math.PI * 2);
      ctx.fillStyle = avecColor(av, isSelected ? 0.1 : 0.05);
      ctx.fill();

      ctx.beginPath();
      ctx.arc(sc.x, sc.y, r + pulse, 0, Math.PI * 2);
      ctx.fillStyle   = avecColor(av, isSelected ? 0.65 : 0.22);
      ctx.fill();
      ctx.strokeStyle = avecColor(av, isSelected ? 0.9 : 0.28);
      ctx.lineWidth   = isSelected ? 0.8 : 0.5;
      ctx.stroke();
    });
  }

  function drawCollapseOrb() {
    if (level !== 2 || !selectedNode) return;
    const np = nodePos[selectedNode.id];
    if (!np) return;
    const sc = toScreen(np.x, np.y);

    const avec = cardData?.nodeDto?.userAvec ?? {
      stability:0.75, friction:0.18, logic:0.82, autonomy:0.88, psi:2.63,
    };
    const { r, g, b } = avecToRgb(avec);
    const col = `${Math.round(r)},${Math.round(g)},${Math.round(b)}`;
    const breathe = Math.sin(t * 1.2) * 4;

    const glo = ctx.createRadialGradient(sc.x, sc.y, 16, sc.x, sc.y, 90 + breathe);
    glo.addColorStop(0, `rgba(${col},0.18)`);
    glo.addColorStop(1, `rgba(${col},0)`);
    ctx.beginPath();
    ctx.arc(sc.x, sc.y, 90 + breathe, 0, Math.PI * 2);
    ctx.fillStyle = glo;
    ctx.fill();

    ctx.beginPath();
    ctx.arc(sc.x, sc.y, 40 + breathe * 0.5, 0, Math.PI * 2);
    ctx.fillStyle   = `rgba(${col},0.07)`;
    ctx.fill();
    ctx.strokeStyle = `rgba(${col},0.18)`;
    ctx.lineWidth   = 0.5;
    ctx.stroke();

    ctx.beginPath();
    ctx.arc(sc.x, sc.y, 26 + breathe * 0.3, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(${col},0.22)`;
    ctx.fill();

    ctx.beginPath();
    ctx.arc(sc.x, sc.y, 14, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(${col},0.78)`;
    ctx.fill();

    ctx.beginPath();
    ctx.arc(sc.x - 5, sc.y - 5, 5, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(${Math.min(255,Math.round(r+45))},${Math.min(255,Math.round(g+45))},${Math.min(255,Math.round(b+45))},0.55)`;
    ctx.fill();

    const corners: [Vec2, number][] = [
      [{ x: 0,   y: 0   }, 0.10],
      [{ x: W(), y: 0   }, 0.07],
      [{ x: 0,   y: H() }, 0.09],
      [{ x: W(), y: H() }, 0.06],
    ];
    corners.forEach(([c, alpha]) => {
      ctx.beginPath();
      ctx.moveTo(sc.x, sc.y);
      ctx.lineTo(c.x, c.y);
      ctx.strokeStyle = `rgba(${col},${alpha})`;
      ctx.lineWidth   = 0.4;
      ctx.setLineDash([2, 6]);
      ctx.stroke();
      ctx.setLineDash([]);
    });

    ctx.textAlign = 'center';
    const ts = cardData?.node?.timestamp ? formatTimestamp(cardData.node.timestamp) : '';
    if (ts) {
      ctx.font      = `11px ${FONT_MONO}`;
      ctx.fillStyle = 'rgba(255,255,255,0.42)';
      ctx.fillText(ts, sc.x, sc.y + 74);
    }
    const tier = cardData?.node?.tier;
    if (tier) {
      ctx.font      = `9px ${FONT_MONO}`;
      ctx.fillStyle = 'rgba(255,255,255,0.2)';
      ctx.fillText(tier.toLowerCase(), sc.x, sc.y + 90);
    }
  }

  function drawHints() {
    if (loading) return;
    ctx.textAlign = 'center';
    ctx.font      = `10px ${FONT_MONO}`;
    const fade = 0.12 + Math.sin(t * 0.7) * 0.06;
    if (level === 0) {
      ctx.fillStyle = `rgba(255,255,255,${fade})`;
      ctx.fillText('pan to explore · tap a wave to enter', W() / 2, H() - 30);
    } else if (level === 1) {
      ctx.fillStyle = `rgba(255,255,255,${fade})`;
      ctx.fillText('tap a moment to descend · tap empty to surface', W() / 2, H() - 30);
    } else if (level === 2 && !cardVisible) {
      ctx.fillStyle = `rgba(255,255,255,${fade * 0.7})`;
      ctx.fillText('tap anywhere to surface', W() / 2, H() - 30);
    }
  }

  // ── Main render loop ────────────────────────────────────────────
  function draw() {
    if (!ctx) { raf = requestAnimationFrame(draw); return; }

    camX     += (targetCamX     - camX)     * LERP;
    camY     += (targetCamY     - camY)     * LERP;
    camScale += (targetCamScale - camScale) * LERP;

    ctx.clearRect(0, 0, W(), H());
    ctx.fillStyle = '#0a0b0e';
    ctx.fillRect(0, 0, W(), H());

    drawStars();

    ctx.save();
    ctx.translate(W() / 2, H() / 2);
    ctx.scale(camScale, camScale);
    ctx.translate(-camX, -camY);
    drawEdges();
    drawSessions();
    drawWaveBoundary();
    drawNodes();
    ctx.restore();

    drawCollapseOrb();
    drawHints();

    if (loading) {
      ctx.textAlign = 'center';
      ctx.font      = `11px ${FONT_MONO}`;
      ctx.fillStyle = `rgba(255,255,255,${0.2 + Math.sin(t * 2) * 0.07})`;
      ctx.fillText('reading the terrain…', W() / 2, H() / 2);
    }
    if (error && !loading) {
      ctx.textAlign = 'center';
      ctx.font      = `11px ${FONT_MONO}`;
      ctx.fillStyle = 'rgba(233,148,58,0.65)';
      ctx.fillText(error.slice(0, 80), W() / 2, H() / 2);
    }

    t += 0.01;
    raf = requestAnimationFrame(draw);
  }

  // ── Resize ───────────────────────────────────────────────────
  function resize() {
    if (!canvas || !container) return;
    const w = container.offsetWidth, h = container.offsetHeight;
    if (w === 0 || h === 0) return;
    canvas.width  = w;
    canvas.height = h;
    layoutConstellation();
    if (level === 0) {
      camX = targetCamX = W() / 2;
      camY = targetCamY = H() / 2;
    }
  }

  // ── Pointer events ─────────────────────────────────────────────
  function onPointerDown(e: MouseEvent) {
    if (level !== 0) return;
    dragging    = true;
    didDrag     = false;
    dragStart   = { x: e.clientX, y: e.clientY };
    panCamStart = { x: camX, y: camY };
  }

  function onPointerMove(e: MouseEvent) {
    if (!dragging || level !== 0) return;
    const dx = e.clientX - dragStart.x;
    const dy = e.clientY - dragStart.y;
    if (Math.abs(dx) > 3 || Math.abs(dy) > 3) didDrag = true;
    camX = targetCamX = panCamStart.x - dx / camScale;
    camY = targetCamY = panCamStart.y - dy / camScale;
  }

  function onPointerUp(e: MouseEvent) {
    dragging = false;
    if (didDrag) return;

    const { x: sx, y: sy } = canvasXY(e);
    const world = toWorld(sx, sy);

    if (level === 2) { surfaceToWave(); return; }

    if (level === 1 && selectedSession) {
      const sessionNodes = graph?.nodes.filter(n => n.sessionId === selectedSession!.id) ?? [];
      for (const n of sessionNodes) {
        const np = nodePos[n.id];
        if (!np) continue;
        if (Math.hypot(world.x - np.x, world.y - np.y) < nodeRadius(n) + 12) {
          descendToCollapse(n);
          return;
        }
      }
      surfaceToConstellation();
      return;
    }

    if (level === 0 && graph) {
      for (const s of graph.sessions) {
        const sp = sessionPos[s.id];
        if (!sp) continue;
        if (Math.hypot(world.x - sp.x, world.y - sp.y) < sessionRadius(s) + 14) {
          descendToWave(s);
          return;
        }
      }
    }
  }

  // ── Health ───────────────────────────────────────────────────
  let healthy = false;
  async function checkHealth() {
    try { await invoke('get_health'); healthy = true; }
    catch { healthy = false; }
  }

  // ── Compose ──────────────────────────────────────────────────
  let composeOpen     = false;
  let composeText     = '';
  let composeSessionId = '';
  let composeLoading  = false;
  let composeError: string | null = null;
  let composeResult: { psi: number } | null = null;

  function openCompose() {
    composeSessionId = selectedSession?.id ?? '';
    composeError = null; composeResult = null;
    composeOpen = true;
  }

  async function submitCompose() {
    if (!composeText.trim()) return;
    composeLoading = true; composeError = null; composeResult = null;
    try {
      const res = await invoke<{ nodeId: string; psi: number; valid: boolean; validationError: string | null }>(
        'store_context', { request: { node: composeText, sessionId: composeSessionId } },
      );
      composeResult = { psi: res.psi };
      composeText = '';
      await loadGraph();
    } catch (err) { composeError = String(err); }
    finally      { composeLoading = false; }
  }

  // ── Calibrate ───────────────────────────────────────────────
  let calibrateOpen  = false;
  let calibSessionId = '';
  let calibStability = 0.5;
  let calibFriction  = 0.2;
  let calibLogic     = 0.8;
  let calibAutonomy  = 0.9;
  let calibTrigger   = 'manual';
  let calibLoading   = false;
  let calibError: string | null = null;

  function openCalibrate() {
    calibSessionId = selectedSession?.id ?? '';
    calibError = null;
    calibrateOpen = true;
  }

  async function submitCalibrate() {
    calibLoading = true; calibError = null;
    try {
      await invoke('calibrate_session', {
        request: {
          sessionId: calibSessionId,
          stability: calibStability,
          friction:  calibFriction,
          logic:     calibLogic,
          autonomy:  calibAutonomy,
          trigger:   calibTrigger,
        },
      });
      calibrateOpen = false;
    } catch (err) { calibError = String(err); }
    finally      { calibLoading = false; }
  }

  // ── Lifecycle ─────────────────────────────────────────────────
  onMount(() => {
    ctx = canvas.getContext('2d')!;
    draw();
    requestAnimationFrame(() => { resize(); loadGraph(); });
    checkHealth();
    const ro = new ResizeObserver(resize);
    ro.observe(container);
    return () => ro.disconnect();
  });

  onDestroy(() => cancelAnimationFrame(raf));
</script>

<div class="weaver-root" bind:this={container}>
  <canvas
    bind:this={canvas}
    on:mousedown={onPointerDown}
    on:mousemove={onPointerMove}
    on:mouseup={onPointerUp}
    on:mouseleave={() => (dragging = false)}
    class:grabbing={dragging}
  ></canvas>

  <nav class="navbar">
    <div class="nav-left">
      {#if level > 0}
        <button class="back-btn" on:click={level === 2 ? surfaceToWave : surfaceToConstellation}>
          ← {level === 2 ? 'wave' : 'constellation'}
        </button>
      {:else}
        <span class="brand">resonantia</span>
      {/if}
    </div>
    <div class="nav-right">
      <span class="status-dot" class:healthy></span>
      <button class="nav-btn" on:click={loadGraph}>refresh</button>
      <button class="nav-btn" on:click={openCalibrate}>calibrate</button>
    </div>
  </nav>

  <CollapseCard
    data={cardData}
    visible={cardVisible}
    on:close={closeCard}
    on:navigate={handleNavigate}
  />

  <button class="compose-btn" on:click={openCompose}>+ compose</button>

  {#if composeOpen}
    <div class="drawer" role="dialog" aria-label="Compose context">
      <div class="drawer-header">
        <span class="drawer-title">compose</span>
        <button class="close-btn" on:click={() => (composeOpen = false)}>✕</button>
      </div>
      <input class="drawer-input" type="text" placeholder="session id" bind:value={composeSessionId} />
      <textarea class="drawer-textarea" placeholder="what happened…" bind:value={composeText} rows="6"></textarea>
      {#if composeError}<p class="drawer-error">{composeError}</p>{/if}
      {#if composeResult}<p class="drawer-success">stored · Ψ {composeResult.psi.toFixed(4)}</p>{/if}
      <div class="drawer-actions">
        <button class="drawer-btn cancel" on:click={() => (composeOpen = false)}>cancel</button>
        <button class="drawer-btn submit" on:click={submitCompose} disabled={composeLoading}>
          {composeLoading ? 'storing…' : 'store'}
        </button>
      </div>
    </div>
  {/if}

  {#if calibrateOpen}
    <div class="drawer" role="dialog" aria-label="Calibrate session">
      <div class="drawer-header">
        <span class="drawer-title">calibrate</span>
        <button class="close-btn" on:click={() => (calibrateOpen = false)}>✕</button>
      </div>
      <input class="drawer-input" type="text" placeholder="session id" bind:value={calibSessionId} />
      <div class="slider-row">
        <span class="slider-label" style="color:{AVEC_HEX.stability}">stability</span>
        <input type="range" min="0" max="1" step="0.01" bind:value={calibStability} class="avec-slider" />
        <span class="slider-val">{calibStability.toFixed(2)}</span>
      </div>
      <div class="slider-row">
        <span class="slider-label" style="color:{AVEC_HEX.friction}">friction</span>
        <input type="range" min="0" max="1" step="0.01" bind:value={calibFriction} class="avec-slider" />
        <span class="slider-val">{calibFriction.toFixed(2)}</span>
      </div>
      <div class="slider-row">
        <span class="slider-label" style="color:{AVEC_HEX.logic}">logic</span>
        <input type="range" min="0" max="1" step="0.01" bind:value={calibLogic} class="avec-slider" />
        <span class="slider-val">{calibLogic.toFixed(2)}</span>
      </div>
      <div class="slider-row">
        <span class="slider-label" style="color:{AVEC_HEX.autonomy}">autonomy</span>
        <input type="range" min="0" max="1" step="0.01" bind:value={calibAutonomy} class="avec-slider" />
        <span class="slider-val">{calibAutonomy.toFixed(2)}</span>
      </div>
      <input class="drawer-input" type="text" placeholder="trigger (e.g. session_start, manual)" bind:value={calibTrigger} />
      {#if calibError}<p class="drawer-error">{calibError}</p>{/if}
      <div class="drawer-actions">
        <button class="drawer-btn cancel" on:click={() => (calibrateOpen = false)}>cancel</button>
        <button class="drawer-btn submit" on:click={submitCalibrate} disabled={calibLoading}>
          {calibLoading ? 'calibrating…' : 'calibrate'}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .weaver-root {
    position: relative;
    width: 100%;
    height: 100vh;
    overflow: hidden;
    background: #0a0b0e;
    font-family: 'Departure Mono', 'Courier New', monospace;
  }

  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    cursor: grab;
    display: block;
  }
  canvas.grabbing { cursor: grabbing; }

  .navbar {
    position: absolute;
    top: 0; left: 0; right: 0;
    height: 46px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 22px;
    background: linear-gradient(to bottom, rgba(10,11,14,0.88) 0%, transparent 100%);
    z-index: 10;
    pointer-events: none;
  }

  .nav-left, .nav-right {
    display: flex;
    align-items: center;
    gap: 8px;
    pointer-events: all;
  }

  .brand {
    font-family: 'Fraunces', Georgia, serif;
    font-weight: 300;
    font-style: italic;
    font-size: 15px;
    color: rgba(255, 255, 255, 0.4);
    letter-spacing: 0.04em;
  }

  .back-btn {
    font-family: 'Departure Mono', monospace;
    font-size: 10px;
    letter-spacing: 0.08em;
    color: rgba(255, 255, 255, 0.35);
    background: transparent;
    border: 0.5px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    padding: 4px 10px;
    cursor: pointer;
    transition: color 0.2s, border-color 0.2s;
  }
  .back-btn:hover {
    color: rgba(255, 255, 255, 0.75);
    border-color: rgba(255, 255, 255, 0.25);
  }

  .status-dot {
    width: 6px; height: 6px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.15);
    transition: background 0.4s;
  }
  .status-dot.healthy { background: #7aaa7a; }

  .nav-btn {
    font-family: 'Departure Mono', monospace;
    font-size: 10px;
    letter-spacing: 0.08em;
    color: rgba(255, 255, 255, 0.3);
    background: transparent;
    border: 0.5px solid rgba(255, 255, 255, 0.08);
    border-radius: 4px;
    padding: 4px 10px;
    cursor: pointer;
    transition: color 0.2s, border-color 0.2s;
  }
  .nav-btn:hover {
    color: rgba(255, 255, 255, 0.75);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .compose-btn {
    position: absolute;
    bottom: 24px; right: 24px;
    font-family: 'Departure Mono', monospace;
    font-size: 11px;
    letter-spacing: 0.1em;
    color: rgba(255, 255, 255, 0.6);
    background: rgba(14, 16, 22, 0.88);
    border: 0.5px solid rgba(255, 255, 255, 0.14);
    border-radius: 6px;
    padding: 10px 18px;
    cursor: pointer;
    z-index: 10;
    transition: all 0.2s;
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }
  .compose-btn:hover {
    color: #fff;
    border-color: rgba(255, 255, 255, 0.28);
    background: rgba(20, 23, 32, 0.95);
  }

  .drawer {
    position: absolute;
    bottom: 70px; left: 50%;
    transform: translateX(-50%);
    width: 360px;
    background: rgba(10, 11, 14, 0.97);
    border: 0.5px solid rgba(255, 255, 255, 0.1);
    border-radius: 14px;
    padding: 20px;
    z-index: 20;
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    font-family: 'Departure Mono', 'Courier New', monospace;
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
  }

  .drawer-title {
    font-family: 'Fraunces', Georgia, serif;
    font-weight: 300;
    font-style: italic;
    font-size: 15px;
    color: rgba(255, 255, 255, 0.55);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.3);
    font-size: 14px;
    cursor: pointer;
    padding: 0;
    transition: color 0.2s;
  }
  .close-btn:hover { color: rgba(255, 255, 255, 0.7); }

  .drawer-input {
    width: 100%; box-sizing: border-box;
    background: rgba(255, 255, 255, 0.04);
    border: 0.5px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 8px 10px;
    color: rgba(255, 255, 255, 0.7);
    font-family: 'Departure Mono', monospace;
    font-size: 11px;
    margin-bottom: 10px;
    outline: none;
    transition: border-color 0.2s;
  }
  .drawer-input:focus { border-color: rgba(255, 255, 255, 0.25); }

  .drawer-textarea {
    width: 100%; box-sizing: border-box;
    background: rgba(255, 255, 255, 0.04);
    border: 0.5px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 8px 10px;
    color: rgba(255, 255, 255, 0.7);
    font-family: 'Departure Mono', monospace;
    font-size: 11px;
    resize: vertical;
    margin-bottom: 10px;
    outline: none;
    transition: border-color 0.2s;
  }
  .drawer-textarea:focus { border-color: rgba(255, 255, 255, 0.25); }

  .slider-row { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }
  .slider-label { font-size: 9px; letter-spacing: 0.1em; text-transform: uppercase; width: 58px; flex-shrink: 0; }
  .avec-slider  { flex: 1; accent-color: rgba(255,255,255,0.4); height: 2px; }
  .slider-val   { font-size: 10px; color: rgba(255,255,255,0.4); width: 32px; text-align: right; flex-shrink: 0; }

  .drawer-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px; }

  .drawer-btn {
    font-family: 'Departure Mono', monospace;
    font-size: 10px;
    letter-spacing: 0.08em;
    padding: 6px 14px;
    border-radius: 5px;
    cursor: pointer;
    transition: all 0.2s;
  }
  .drawer-btn.cancel {
    background: transparent;
    border: 0.5px solid rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.3);
  }
  .drawer-btn.submit {
    background: rgba(255, 255, 255, 0.06);
    border: 0.5px solid rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 0.8);
  }
  .drawer-btn.submit:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.35);
  }
  .drawer-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .drawer-error   { font-size: 10px; color: rgba(233,148,58,0.8);  margin: 6px 0 0; }
  .drawer-success { font-size: 10px; color: rgba(122,170,122,0.9); margin: 6px 0 0; }
</style>
