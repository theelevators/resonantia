<script lang="ts">
  import { createEventDispatcher, onDestroy, tick } from 'svelte';
  import { fade } from 'svelte/transition';

  export let open = false;

  const dispatch = createEventDispatcher<{
    complete: {
      name: string;
      d1: 'light' | 'sound';
      d2: string;
      d3: string;
      avec: { stability: number; friction: number; logic: number; autonomy: number; psi: number };
      avecBase: { stability: number; friction: number; logic: number; autonomy: number; psi: number };
      metrics: AdventureSignalMetrics;
    };
    skip: void;
  }>();

  type AdventureState =
    | 'entry'
    | 'decision1'
    | 'naming'
    | 'decision2'
    | 'decision3'
    | 'silence'
    | 'reveal'
    | 'mapping'
    | 'cta'
    | 'confirmed';

  type ChoiceStageId = 'decision1' | 'decision2' | 'decision3' | 'cta';

  type StageStyle = 'decisive' | 'explored' | 'wavering';

  type ChoiceTrace = {
    startedAt: number;
    endedAt: number | null;
    firstInteractionAt: number | null;
    clickAt: number | null;
    selectedOption: string | null;
    activeHoverOption: string | null;
    activeHoverStartedAt: number | null;
    hoverMsByOption: Record<string, number>;
    enterCountByOption: Record<string, number>;
    moveCountByOption: Record<string, number>;
    switchCount: number;
  };

  type StageSignal = {
    stage: ChoiceStageId;
    selectedOption: string | null;
    stareMs: number;
    decisionMs: number;
    hoverMsByOption: Record<string, number>;
    hoverTotalMs: number;
    switches: number;
    optionVisits: number;
    moveCount: number;
    style: StageStyle;
    hesitation: number;
  };

  type AdventureSignalMetrics = {
    totalDurationMs: number;
    explorationScore: number;
    hesitationScore: number;
    decisivenessScore: number;
    engagementScore: number;
    weights: {
      stability: number;
      friction: number;
      logic: number;
      autonomy: number;
    };
    stages: Record<ChoiceStageId, StageSignal>;
    naming: {
      dwellMs: number;
      firstInputLatencyMs: number;
      inputChanges: number;
      backspaces: number;
    };
  };

  const TRACKED_STAGES: ChoiceStageId[] = ['decision1', 'decision2', 'decision3', 'cta'];

  const DEFAULT_STAGE_OPTIONS: Record<ChoiceStageId, string[]> = {
    decision1: ['light', 'sound'],
    decision2: ['complete', 'open', 'sync', 'resist'],
    decision3: ['merge', 'observe'],
    cta: ['plant', 'skip'],
  };

  let state: AdventureState = 'entry';
  let d1: 'light' | 'sound' | null = null;
  let d2: string | null = null;
  let d3: string | null = null;
  let chosenName = '';
  let nameInput: HTMLInputElement | null = null;
  let mappingStep = 0;
  let mappingCanAdvance = false;
  let mappingInterval: ReturnType<typeof setInterval> | null = null;
  let hoverZone: 'light' | 'sound' | null = null;
  let revealCanAdvance = false;
  let revealHintVisible = false;
  let lightPointer = { x: 68, y: 50 };
  let soundPointer = { x: 32, y: 50 };
  let wasOpen = false;
  let avecBase = computeBaseAvec();
  let signalMetrics = emptyAdventureSignalMetrics();
  let avec = applySignalWeights(avecBase, signalMetrics);
  let telemetryVersion = 0;
  let adventureStartedAt = 0;
  let previousState: AdventureState | null = null;
  let d1HoverLightMs = 0;
  let d1HoverSoundMs = 0;
  let stageStyleSummary = '';
  let shiftSummary = '';
  let hoverStory = '';
  let hesitationStory = '';
  let explorationStory = '';
  let engagementStory = '';
  let weightingStory = '';
  let nameStageStartedAt = 0;
  let nameFirstInputAt: number | null = null;
  let nameInputChanges = 0;
  let nameBackspaces = 0;
  let stageTraces = buildChoiceTraceMap();
  let mappingLines: string[] = [];

  const DIMENSION_LABELS = {
    stability: 'grounding',
    friction: 'wear',
    logic: 'clarity',
    autonomy: 'self-trust',
  } as const;

  type DimensionKey = keyof typeof DIMENSION_LABELS;

  const timeouts = new Set<ReturnType<typeof setTimeout>>();

  function nowMs() {
    return typeof performance !== 'undefined' ? performance.now() : Date.now();
  }

  function clamp01(value: number) {
    return Math.max(0, Math.min(1, value));
  }

  function clamp(value: number, min: number, max: number) {
    return Math.max(min, Math.min(max, value));
  }

  function round2(value: number) {
    return +value.toFixed(2);
  }

  function bumpTelemetry() {
    telemetryVersion += 1;
  }

  function buildChoiceTrace(options: string[]): ChoiceTrace {
    const hoverMsByOption: Record<string, number> = {};
    const enterCountByOption: Record<string, number> = {};
    const moveCountByOption: Record<string, number> = {};

    options.forEach((option) => {
      hoverMsByOption[option] = 0;
      enterCountByOption[option] = 0;
      moveCountByOption[option] = 0;
    });

    return {
      startedAt: 0,
      endedAt: null,
      firstInteractionAt: null,
      clickAt: null,
      selectedOption: null,
      activeHoverOption: null,
      activeHoverStartedAt: null,
      hoverMsByOption,
      enterCountByOption,
      moveCountByOption,
      switchCount: 0,
    };
  }

  function buildChoiceTraceMap() {
    return {
      decision1: buildChoiceTrace(DEFAULT_STAGE_OPTIONS.decision1),
      decision2: buildChoiceTrace(DEFAULT_STAGE_OPTIONS.decision2),
      decision3: buildChoiceTrace(DEFAULT_STAGE_OPTIONS.decision3),
      cta: buildChoiceTrace(DEFAULT_STAGE_OPTIONS.cta),
    } as Record<ChoiceStageId, ChoiceTrace>;
  }

  function emptyStageSignal(stage: ChoiceStageId): StageSignal {
    return {
      stage,
      selectedOption: null,
      stareMs: 0,
      decisionMs: 0,
      hoverMsByOption: {},
      hoverTotalMs: 0,
      switches: 0,
      optionVisits: 0,
      moveCount: 0,
      style: 'decisive',
      hesitation: 0,
    };
  }

  function emptyAdventureSignalMetrics(): AdventureSignalMetrics {
    return {
      totalDurationMs: 0,
      explorationScore: 0,
      hesitationScore: 0,
      decisivenessScore: 1,
      engagementScore: 0,
      weights: {
        stability: 0,
        friction: 0,
        logic: 0,
        autonomy: 0,
      },
      stages: {
        decision1: emptyStageSignal('decision1'),
        decision2: emptyStageSignal('decision2'),
        decision3: emptyStageSignal('decision3'),
        cta: emptyStageSignal('cta'),
      },
      naming: {
        dwellMs: 0,
        firstInputLatencyMs: 0,
        inputChanges: 0,
        backspaces: 0,
      },
    };
  }

  function isChoiceStage(stage: AdventureState | null): stage is ChoiceStageId {
    return stage !== null && TRACKED_STAGES.includes(stage as ChoiceStageId);
  }

  function activeStageOptions(stage: ChoiceStageId): string[] {
    if (stage === 'decision2') {
      return d1 === 'light' ? ['complete', 'open'] : ['sync', 'resist'];
    }

    return DEFAULT_STAGE_OPTIONS[stage];
  }

  function ensureOptionRecords(trace: ChoiceTrace, option: string) {
    if (!(option in trace.hoverMsByOption)) trace.hoverMsByOption[option] = 0;
    if (!(option in trace.enterCountByOption)) trace.enterCountByOption[option] = 0;
    if (!(option in trace.moveCountByOption)) trace.moveCountByOption[option] = 0;
  }

  function settleActiveHover(trace: ChoiceTrace, settledAt = nowMs()) {
    if (!trace.activeHoverOption || trace.activeHoverStartedAt === null) return;

    ensureOptionRecords(trace, trace.activeHoverOption);
    trace.hoverMsByOption[trace.activeHoverOption] += Math.max(0, settledAt - trace.activeHoverStartedAt);
    trace.activeHoverOption = null;
    trace.activeHoverStartedAt = null;
  }

  function beginChoiceStage(stage: ChoiceStageId) {
    stageTraces[stage] = buildChoiceTrace(activeStageOptions(stage));
    stageTraces[stage].startedAt = nowMs();
    bumpTelemetry();
  }

  function endChoiceStage(stage: ChoiceStageId) {
    const trace = stageTraces[stage];
    if (!trace.startedAt) return;

    const endedAt = nowMs();
    settleActiveHover(trace, endedAt);
    if (trace.endedAt === null) {
      trace.endedAt = endedAt;
    }
    bumpTelemetry();
  }

  function noteChoiceHoverEnter(stage: ChoiceStageId, option: string) {
    if (state !== stage) return;

    const trace = stageTraces[stage];
    if (!trace.startedAt) {
      beginChoiceStage(stage);
    }

    const at = nowMs();
    if (trace.firstInteractionAt === null) trace.firstInteractionAt = at;
    ensureOptionRecords(trace, option);

    if (trace.activeHoverOption && trace.activeHoverOption !== option) {
      settleActiveHover(trace, at);
      trace.switchCount += 1;
    }

    if (trace.activeHoverOption !== option) {
      trace.activeHoverOption = option;
      trace.activeHoverStartedAt = at;
      trace.enterCountByOption[option] += 1;
    }

    bumpTelemetry();
  }

  function noteChoiceHoverLeave(stage: ChoiceStageId, option: string) {
    if (state !== stage) return;

    const trace = stageTraces[stage];
    if (trace.activeHoverOption !== option) return;

    settleActiveHover(trace, nowMs());
    bumpTelemetry();
  }

  function noteChoiceHoverMove(stage: ChoiceStageId, option: string) {
    if (state !== stage) return;

    const trace = stageTraces[stage];
    if (!trace.startedAt) {
      beginChoiceStage(stage);
    }

    const at = nowMs();
    if (trace.firstInteractionAt === null) trace.firstInteractionAt = at;
    ensureOptionRecords(trace, option);
    trace.moveCountByOption[option] += 1;
    bumpTelemetry();
  }

  function noteChoiceClick(stage: ChoiceStageId, option: string) {
    if (state !== stage) return;

    const trace = stageTraces[stage];
    if (!trace.startedAt) {
      beginChoiceStage(stage);
    }

    const at = nowMs();
    if (trace.firstInteractionAt === null) trace.firstInteractionAt = at;
    ensureOptionRecords(trace, option);

    if (trace.activeHoverOption && trace.activeHoverOption !== option) {
      settleActiveHover(trace, at);
      trace.switchCount += 1;
    }

    if (trace.activeHoverOption === option) {
      settleActiveHover(trace, at);
    }

    trace.selectedOption = option;
    trace.clickAt = at;
    trace.endedAt = at;
    bumpTelemetry();
  }

  function stageStyle(optionVisits: number, switches: number, hesitation: number): StageStyle {
    if (switches >= 2 || hesitation > 0.62) return 'wavering';
    if (optionVisits > 1 || switches === 1) return 'explored';
    return 'decisive';
  }

  function computeWeightShifts(exploration: number, hesitation: number, decisiveness: number, engagement: number) {
    const stability = clamp(((decisiveness * 0.62 + engagement * 0.2) - (hesitation * 0.55 + exploration * 0.16)) * 0.2, -0.12, 0.12);
    const friction = clamp(((hesitation * 0.62 + exploration * 0.34) - (decisiveness * 0.5 + engagement * 0.22)) * 0.22, -0.14, 0.14);
    const logic = clamp(((decisiveness * 0.52 + engagement * 0.34) - hesitation * 0.34) * 0.18, -0.1, 0.1);
    const autonomy = clamp(((exploration * 0.56 + decisiveness * 0.25 + engagement * 0.19) - hesitation * 0.32) * 0.2, -0.12, 0.12);

    return {
      stability: round2(stability),
      friction: round2(friction),
      logic: round2(logic),
      autonomy: round2(autonomy),
    };
  }

  function buildStageSignal(stage: ChoiceStageId): StageSignal {
    const trace = stageTraces[stage];
    if (!trace.startedAt) return emptyStageSignal(stage);

    const at = nowMs();
    const hoverMsByOption = { ...trace.hoverMsByOption };
    if (trace.activeHoverOption && trace.activeHoverStartedAt !== null) {
      ensureOptionRecords(trace, trace.activeHoverOption);
      hoverMsByOption[trace.activeHoverOption] += Math.max(0, at - trace.activeHoverStartedAt);
    }

    const endAt = trace.clickAt ?? trace.endedAt ?? at;
    const firstInteractionAt = trace.firstInteractionAt ?? endAt;
    const decisionMs = Math.max(0, endAt - trace.startedAt);
    const stareMs = Math.max(0, firstInteractionAt - trace.startedAt);
    const hoverTotalMs = Object.values(hoverMsByOption).reduce((sum, value) => sum + value, 0);
    const optionVisits = Object.values(trace.enterCountByOption).filter((value) => value > 0).length;
    const moveCount = Object.values(trace.moveCountByOption).reduce((sum, value) => sum + value, 0);

    const hesitation = clamp01(
      Math.min(1, decisionMs / 6200) * 0.52
      + Math.min(1, stareMs / 2600) * 0.3
      + Math.min(1, trace.switchCount / 4) * 0.18,
    );

    return {
      stage,
      selectedOption: trace.selectedOption,
      stareMs,
      decisionMs,
      hoverMsByOption,
      hoverTotalMs,
      switches: trace.switchCount,
      optionVisits,
      moveCount,
      style: stageStyle(optionVisits, trace.switchCount, hesitation),
      hesitation: round2(hesitation),
    };
  }

  function deriveSignalMetrics(_telemetryVersion: number): AdventureSignalMetrics {
    const d1Signal = buildStageSignal('decision1');
    const d2Signal = buildStageSignal('decision2');
    const d3Signal = buildStageSignal('decision3');
    const ctaSignal = buildStageSignal('cta');
    const decisionStages = [d1Signal, d2Signal, d3Signal];
    const safeCount = decisionStages.length || 1;

    const explorationScore = clamp01(
      (decisionStages.reduce((sum, stage) => sum + (stage.optionVisits > 1 ? 1 : 0), 0) / safeCount) * 0.68
      + (decisionStages.reduce((sum, stage) => sum + Math.min(1, stage.switches / 3), 0) / safeCount) * 0.32,
    );

    const hesitationScore = clamp01(
      decisionStages.reduce((sum, stage) => sum + stage.hesitation, 0) / safeCount,
    );

    const decisivenessScore = clamp01(
      1
      - hesitationScore * 0.74
      + decisionStages.reduce((sum, stage) => sum + (stage.style === 'decisive' ? 0.18 : stage.style === 'explored' ? 0.1 : 0), 0) / safeCount,
    );

    const engagementScore = clamp01(
      (decisionStages.reduce((sum, stage) => sum + Math.min(1, stage.hoverTotalMs / 2200), 0) / safeCount) * 0.45
      + (decisionStages.reduce((sum, stage) => sum + Math.min(1, stage.moveCount / 24), 0) / safeCount) * 0.35
      + (nameInputChanges > 0 ? 0.2 : 0),
    );

    const namingDwellMs = nameStageStartedAt ? Math.max(0, (state === 'naming' ? nowMs() : (nameFirstInputAt ?? nowMs())) - nameStageStartedAt) : 0;
    const firstInputLatencyMs = nameStageStartedAt && nameFirstInputAt
      ? Math.max(0, nameFirstInputAt - nameStageStartedAt)
      : 0;

    const weights = computeWeightShifts(explorationScore, hesitationScore, decisivenessScore, engagementScore);

    return {
      totalDurationMs: adventureStartedAt ? Math.max(0, nowMs() - adventureStartedAt) : 0,
      explorationScore: round2(explorationScore),
      hesitationScore: round2(hesitationScore),
      decisivenessScore: round2(decisivenessScore),
      engagementScore: round2(engagementScore),
      weights,
      stages: {
        decision1: d1Signal,
        decision2: d2Signal,
        decision3: d3Signal,
        cta: ctaSignal,
      },
      naming: {
        dwellMs: namingDwellMs,
        firstInputLatencyMs,
        inputChanges: nameInputChanges,
        backspaces: nameBackspaces,
      },
    };
  }

  function applySignalWeights(base: { stability: number; friction: number; logic: number; autonomy: number; psi: number }, metrics: AdventureSignalMetrics) {
    const stability = clamp(base.stability + metrics.weights.stability, 0.1, 1);
    const friction = clamp(base.friction + metrics.weights.friction, 0.1, 1);
    const logic = clamp(base.logic + metrics.weights.logic, 0.1, 1);
    const autonomy = clamp(base.autonomy + metrics.weights.autonomy, 0.1, 1);
    const psi = round2(stability + (1 - friction) + logic + autonomy);

    return {
      stability: round2(stability),
      friction: round2(friction),
      logic: round2(logic),
      autonomy: round2(autonomy),
      psi,
    };
  }

  function formatDuration(ms: number) {
    const seconds = ms / 1000;
    if (seconds >= 10) return `${seconds.toFixed(1)}s`;
    return `${seconds.toFixed(2)}s`;
  }

  function formatSigned(value: number) {
    return `${value >= 0 ? '+' : ''}${value.toFixed(2)}`;
  }

  function styleLabel(style: StageStyle) {
    if (style === 'decisive') return 'decisive';
    if (style === 'explored') return 'explored';
    return 'wavering';
  }

  function compareDecisionPull(lightMs: number, soundMs: number) {
    const delta = Math.abs(lightMs - soundMs);
    if (lightMs === 0 && soundMs === 0) {
      return 'you moved almost instantly, so the opening read as instinctive trust.';
    }

    if (delta < 420) {
      return 'you held both signals close before choosing, so the field recorded genuine discernment.';
    }

    if (lightMs > soundMs) {
      return 'you stayed longer with the glow, so grounding and clarity had more early gravity.';
    }

    return 'you stayed longer with the pulse, so self-trust shaped the opening tone.';
  }

  function hesitationReading(score: number) {
    if (score < 0.28) return 'low hesitation: this moment carried a clear through-line.';
    if (score < 0.58) return 'measured hesitation: you weighed paths instead of drifting.';
    return 'high hesitation: the field marked this as an edge with emotional weight.';
  }

  function explorationReading(score: number) {
    if (score < 0.3) return 'focused exploration: you chose depth over sampling every branch.';
    if (score < 0.62) return 'balanced exploration: you sampled enough to test the signal before committing.';
    return 'wide exploration: you actively searched for fit before you landed.';
  }

  function engagementReading(score: number) {
    if (score < 0.3) return 'quiet engagement: subtle movement, steady intent.';
    if (score < 0.62) return 'present engagement: the interaction stayed alive throughout the sequence.';
    return 'high engagement: your attention continuously shaped the field.';
  }

  function later(fn: () => void, delay: number) {
    const id = setTimeout(() => {
      timeouts.delete(id);
      fn();
    }, delay);
    timeouts.add(id);
    return id;
  }

  function clearFlowTimers() {
    for (const timeout of timeouts) {
      clearTimeout(timeout);
    }
    timeouts.clear();
  }

  function clearMappingInterval() {
    if (!mappingInterval) return;
    clearInterval(mappingInterval);
    mappingInterval = null;
  }

  function resetAdventure() {
    clearFlowTimers();
    clearMappingInterval();
    state = 'entry';
    previousState = null;
    d1 = null;
    d2 = null;
    d3 = null;
    chosenName = '';
    mappingStep = 0;
    mappingCanAdvance = false;
    hoverZone = null;
    revealCanAdvance = false;
    revealHintVisible = false;
    lightPointer = { x: 68, y: 50 };
    soundPointer = { x: 32, y: 50 };
    stageTraces = buildChoiceTraceMap();
    nameStageStartedAt = 0;
    nameFirstInputAt = null;
    nameInputChanges = 0;
    nameBackspaces = 0;
    signalMetrics = emptyAdventureSignalMetrics();
    telemetryVersion = 0;
  }

  function startAdventure() {
    resetAdventure();
    adventureStartedAt = nowMs();
    bumpTelemetry();
    later(() => advance('decision1'), 6200);
  }

  function advance(next: AdventureState, delay = 0) {
    if (delay <= 0) {
      state = next;
      return;
    }

    later(() => {
      state = next;
    }, delay);
  }

  $: if (open && !wasOpen) {
    wasOpen = true;
    startAdventure();
  }

  $: if (!open && wasOpen) {
    wasOpen = false;
    clearFlowTimers();
    clearMappingInterval();
  }

  function handleStateTransition(prev: AdventureState | null, next: AdventureState) {
    if (isChoiceStage(prev)) {
      endChoiceStage(prev);
    }

    if (isChoiceStage(next)) {
      beginChoiceStage(next);
    }

    if (next === 'naming') {
      nameStageStartedAt = nowMs();
      nameFirstInputAt = null;
      nameInputChanges = 0;
      nameBackspaces = 0;
      bumpTelemetry();
    }
  }

  $: if (state !== previousState) {
    handleStateTransition(previousState, state);
    previousState = state;
  }

  $: signalMetrics = deriveSignalMetrics(telemetryVersion);
  $: avecBase = computeBaseAvec();
  $: avec = applySignalWeights(avecBase, signalMetrics);

  $: d1HoverLightMs = signalMetrics.stages.decision1.hoverMsByOption.light ?? 0;
  $: d1HoverSoundMs = signalMetrics.stages.decision1.hoverMsByOption.sound ?? 0;
  $: stageStyleSummary = `d1 ${styleLabel(signalMetrics.stages.decision1.style)} · d2 ${styleLabel(signalMetrics.stages.decision2.style)} · d3 ${styleLabel(signalMetrics.stages.decision3.style)}`;
  $: shiftSummary = `grounding ${formatSigned(signalMetrics.weights.stability)} · wear ${formatSigned(signalMetrics.weights.friction)} · clarity ${formatSigned(signalMetrics.weights.logic)} · self-trust ${formatSigned(signalMetrics.weights.autonomy)}`;
  $: hoverStory = compareDecisionPull(d1HoverLightMs, d1HoverSoundMs);
  $: hesitationStory = hesitationReading(signalMetrics.hesitationScore);
  $: explorationStory = explorationReading(signalMetrics.explorationScore);
  $: engagementStory = engagementReading(signalMetrics.engagementScore);
  $: weightingStory = `those patterns produced this weighting: ${shiftSummary}.`;

  onDestroy(() => {
    clearFlowTimers();
    clearMappingInterval();
  });

  function handleD1ZoneEnter(zone: 'light' | 'sound') {
    hoverZone = zone;
    noteChoiceHoverEnter('decision1', zone);
  }

  function handleD1ZoneLeave(zone: 'light' | 'sound') {
    if (hoverZone === zone) hoverZone = null;
    noteChoiceHoverLeave('decision1', zone);
  }

  function handleD1ZoneMove(zone: 'light' | 'sound', e: PointerEvent) {
    trackZonePointer(zone, e);
    noteChoiceHoverMove('decision1', zone);
  }

  function handleNameInput() {
    if (!nameStageStartedAt) return;
    if (nameFirstInputAt === null) {
      nameFirstInputAt = nowMs();
    }
    nameInputChanges += 1;
    bumpTelemetry();
  }

  function pickD1(choice: 'light' | 'sound') {
    noteChoiceClick('decision1', choice);
    d1 = choice;
    hoverZone = null;
    advance('naming', 900);
  }

  async function submitName() {
    if (!chosenName.trim()) return;
    advance('decision2', 550);
  }

  function handleNameKey(e: KeyboardEvent) {
    if (e.key === 'Backspace') {
      nameBackspaces += 1;
      bumpTelemetry();
    }

    if (e.key === 'Enter') submitName();
  }

  function pickD2(choice: string) {
    noteChoiceClick('decision2', choice);
    d2 = choice;
    advance('decision3', 1000);
  }

  function enterRevealState() {
    state = 'reveal';
    mappingStep = 0;
    revealCanAdvance = false;
    revealHintVisible = false;
    later(() => {
      revealCanAdvance = true;
    }, 3000);
    later(() => {
      revealHintVisible = true;
    }, 5200);
  }

  function pickD3(choice: string) {
    noteChoiceClick('decision3', choice);
    d3 = choice;
    state = 'silence';
    revealCanAdvance = false;
    revealHintVisible = false;
    later(() => enterRevealState(), 2400);
  }

  function startMapping() {
    if (state !== 'reveal') return;

    clearMappingInterval();
    state = 'mapping';
    mappingStep = 0;
    mappingCanAdvance = false;
    revealHintVisible = false;
    revealCanAdvance = false;

    mappingInterval = setInterval(() => {
      mappingStep++;
      if (mappingStep >= mappingLines.length) {
        clearMappingInterval();
        later(() => {
          mappingCanAdvance = true;
        }, 850);
      }
    }, 900);
  }

  function advanceFromMapping() {
    if (state !== 'mapping' || !mappingCanAdvance || mappingStep < mappingLines.length) return;
    state = 'cta';
    mappingCanAdvance = false;
  }

  function handleRevealContinue() {
    if (state === 'reveal' && revealCanAdvance) {
      startMapping();
    }
  }

  function trackZonePointer(zone: 'light' | 'sound', e: PointerEvent) {
    const el = e.currentTarget as HTMLElement | null;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const x = ((e.clientX - rect.left) / rect.width) * 100;
    const y = ((e.clientY - rect.top) / rect.height) * 100;
    const point = {
      x: Math.min(100, Math.max(0, x)),
      y: Math.min(100, Math.max(0, y)),
    };

    if (zone === 'light') lightPointer = point;
    else soundPointer = point;
  }

  function computeBaseAvec() {
    let stability = 0.65;
    let friction = 0.45;
    let logic = 0.65;
    let autonomy = 0.6;

    if (d1 === 'light') {
      stability += 0.12;
      logic += 0.1;
    } else {
      autonomy += 0.18;
      friction -= 0.1;
    }

    if (d2 === 'complete' || d2 === 'sync') {
      stability += 0.08;
      logic += 0.06;
    } else {
      autonomy += 0.1;
      friction -= 0.08;
    }

    if (d3 === 'merge') {
      autonomy += 0.12;
      friction -= 0.05;
    } else if (d3) {
      stability += 0.09;
    }

    stability = Math.min(1, +stability.toFixed(2));
    friction = Math.max(0.1, Math.min(1, +friction.toFixed(2)));
    logic = Math.min(1, +logic.toFixed(2));
    autonomy = Math.min(1, +autonomy.toFixed(2));
    const psi = +(stability + (1 - friction) + logic + autonomy).toFixed(2);

    return { stability, friction, logic, autonomy, psi };
  }

  function attractor(): string {
    if (d1 === 'light' && (d2 === 'complete' || d2 === 'sync')) return 'structured openness';
    if (d1 === 'light') return 'precise self-trust';
    if (d1 === 'sound' && d2 === 'sync') return 'resonant flow';
    if (d1 === 'sound') return 'fluid independence';
    return 'open field';
  }

  $: mappingLines = [
    d1 === 'light'
      ? 'moved toward the glow — grounding + clarity rose'
      : 'followed the pulse — self-trust rose',
    `named it "${chosenName.trim() || '—'}" — a fixed point forms`,
    d2 === 'complete'
      ? 'completed the shape — grounding rose'
      : d2 === 'open'
        ? 'left it open — openness held'
        : d2 === 'sync'
          ? 'synced with the rhythm — self-trust rose'
          : d2 === 'resist'
            ? 'held your edge — self-trust held steady'
            : '',
    d3 === 'merge' ? 'stepped inside — self-trust rose' : 'stayed and witnessed — grounding rose',
  ].filter(Boolean);

  $: rankedDimensions = ([
    { key: 'stability', value: avec.stability },
    { key: 'friction', value: avec.friction },
    { key: 'logic', value: avec.logic },
    { key: 'autonomy', value: avec.autonomy },
  ] as { key: DimensionKey; value: number }[]).sort((a, b) => b.value - a.value);

  $: resolvedName = chosenName.trim() || 'unnamed';
  $: dominantSummary = `${DIMENSION_LABELS[rankedDimensions[0].key]} + ${DIMENSION_LABELS[rankedDimensions[1].key]}`;

  $: decisionEffects = [
    {
      choice: d1 === 'light' ? 'moved toward the glow' : 'followed the pulse',
      impact: d1 === 'light' ? 'signal settled toward grounding and clarity' : 'signal opened toward self-trust',
    },
    {
      choice: `named it "${resolvedName}"`,
      impact: 'set a return point in memory',
    },
    {
      choice:
        d2 === 'complete'
          ? 'completed the shape'
          : d2 === 'open'
            ? 'left the shape open'
            : d2 === 'sync'
              ? 'synced with the rhythm'
              : 'held your own edge',
      impact:
        d2 === 'complete'
          ? 'grounding deepened'
          : d2 === 'open'
            ? 'openness stayed alive'
            : d2 === 'sync'
              ? 'self-trust and grounding moved together'
              : 'independence strengthened',
    },
    {
      choice: d3 === 'merge' ? 'stepped inside' : 'stayed and witnessed',
      impact: d3 === 'merge' ? 'self-trust rose, wear softened' : 'grounding deepened',
    },
  ];

  $: explanationWhatHappened = 'In a few choices, you revealed a pattern. Resonantia translated those moves into a memory shape you can return to.';
  $: explanationName =
    resolvedName === 'unnamed'
      ? 'You left it unnamed, so this node keeps a quiet placeholder for now.'
      : `The name "${resolvedName}" is a landmark. It helps you and future sessions find this moment again.`;
  $: explanationNotice = `Your signal leaned toward ${dominantSummary}, settling as "${attractor()}".`;
  $: explanationRelation = 'When you plant this, it joins your real constellation with every other Resonantia memory node. You can revisit it, connect it, and collapse it into new insight.';

  $: if (state === 'naming') focusNameInput();

  async function focusNameInput() {
    await tick();
    nameInput?.focus();
  }

  function plantMemory() {
    noteChoiceClick('cta', 'plant');
    state = 'confirmed';
    const metricsSnapshot = deriveSignalMetrics(telemetryVersion);
    const baseSnapshot = computeBaseAvec();
    const weightedSnapshot = applySignalWeights(baseSnapshot, metricsSnapshot);
    const result = {
      name: chosenName.trim() || 'unnamed',
      d1: d1 ?? 'light',
      d2: d2 ?? 'complete',
      d3: d3 ?? 'merge',
      avec: weightedSnapshot,
      avecBase: baseSnapshot,
      metrics: metricsSnapshot,
    };

    later(() => {
      dispatch('complete', result);
    }, 1100);
  }

  function handleSkip() {
    noteChoiceClick('cta', 'skip');
    dispatch('skip');
  }

  const STARS = Array.from({ length: 42 }, (_, i) => ({
    x: ((i * 37 + 11) % 97) + 1.5,
    y: ((i * 53 + 7) % 94) + 2.5,
    s: 0.8 + ((i * 17) % 8) * 0.22,
    d: ((i * 41) % 9) + 5,
    phase: ((i * 29) % 100) / 100,
    layer: i % 3,
  }));
</script>

{#if open}
  <div
    class="adv"
    class:theme-light={d1 === 'light'}
    class:theme-sound={d1 === 'sound'}
    role="dialog"
    aria-modal="true"
    aria-label="Resonantia adventure onboarding"
    transition:fade={{ duration: 1200 }}
  >
    <div class="ambient-nebula" aria-hidden="true"></div>
    <div class="vignette" aria-hidden="true"></div>

    <div class="particles" aria-hidden="true">
      {#each STARS as star}
        <div
          class="star"
          data-layer={star.layer}
          style="left:{star.x}%;top:{star.y}%;width:{star.s}px;height:{star.s}px;animation-duration:{star.d}s;animation-delay:{-star.phase * star.d}s"
        ></div>
      {/each}
    </div>

    {#if state === 'entry'}
      <div class="screen" transition:fade={{ duration: 760 }}>
        <p class="line" style="--d:0.16s">you arrive in a place you did not plan, but somehow recognize</p>
        <p class="line muted" style="--d:1.7s">quiet enough to hear your own signal.</p>
        <p class="line muted" style="--d:3.05s">something answers back.</p>
      </div>
    {/if}

    {#if state === 'decision1'}
      <div class="screen d1-screen" transition:fade={{ duration: 600 }}>
        <button
          class="zone zone-light"
          class:zone-hovered={hoverZone === 'light'}
          style="--mx:{lightPointer.x}%;--my:{lightPointer.y}%"
          on:pointerenter={() => handleD1ZoneEnter('light')}
          on:pointerleave={() => handleD1ZoneLeave('light')}
          on:pointermove={(e) => handleD1ZoneMove('light', e)}
          on:click={() => pickD1('light')}
          aria-label="move toward the light"
        >
          <div class="light-glow"></div>
          <div class="light-flare"></div>
        </button>

        <button
          class="zone zone-sound"
          class:zone-hovered={hoverZone === 'sound'}
          style="--mx:{soundPointer.x}%;--my:{soundPointer.y}%"
          on:pointerenter={() => handleD1ZoneEnter('sound')}
          on:pointerleave={() => handleD1ZoneLeave('sound')}
          on:pointermove={(e) => handleD1ZoneMove('sound', e)}
          on:click={() => pickD1('sound')}
          aria-label="listen to the pulse"
        >
          <div class="pulse-ring"></div>
          <div class="pulse-ring" style="animation-delay:-1.1s"></div>
          <div class="pulse-ring" style="animation-delay:-2.2s"></div>
          <div class="pulse-ring" style="animation-delay:-3.3s"></div>
        </button>

        <div class="d1-caption" aria-hidden="true">
          <p class="line" style="--d:0.2s">something is reaching for you</p>
          <p class="line muted" style="--d:1.35s">move toward the glow, or follow the pulse.</p>
        </div>
      </div>
    {/if}

    {#if state === 'naming'}
      <div class="screen" transition:fade={{ duration: 700 }}>
        {#if d1 === 'light'}
          <p class="line" style="--d:0.16s">you reached it first.</p>
          <p class="line muted" style="--d:0.98s">give it a name.</p>
        {:else}
          <p class="line" style="--d:0.16s">it reached you first.</p>
          <p class="line muted" style="--d:0.98s">give it a name.</p>
        {/if}

        <div class="name-wrap" style="--d:1.5s">
          <input
            bind:this={nameInput}
            bind:value={chosenName}
            on:input={handleNameInput}
            on:keydown={handleNameKey}
            class="name-input"
            type="text"
            autocomplete="off"
            spellcheck={false}
            maxlength={32}
            placeholder="…"
          />
          {#if chosenName.trim()}
            <button class="name-confirm" on:click={submitName} aria-label="confirm name">→</button>
          {/if}
        </div>
      </div>
    {/if}

    {#if state === 'decision2'}
      <div class="screen" transition:fade={{ duration: 520 }}>
        {#if d1 === 'light'}
          <p class="line" style="--d:0.2s">the shape is almost complete</p>
          <p class="line muted" style="--d:1.2s">one edge is still open</p>
          <div class="choice-row" style="--d:2.9s">
            <button
              class="choice"
              on:pointerenter={() => noteChoiceHoverEnter('decision2', 'complete')}
              on:pointerleave={() => noteChoiceHoverLeave('decision2', 'complete')}
              on:pointermove={() => noteChoiceHoverMove('decision2', 'complete')}
              on:click={() => pickD2('complete')}
            >
              complete it
            </button>
            <span class="choice-div" aria-hidden="true"></span>
            <button
              class="choice"
              on:pointerenter={() => noteChoiceHoverEnter('decision2', 'open')}
              on:pointerleave={() => noteChoiceHoverLeave('decision2', 'open')}
              on:pointermove={() => noteChoiceHoverMove('decision2', 'open')}
              on:click={() => pickD2('open')}
            >
              leave it open
            </button>
          </div>
        {:else}
          <p class="line" style="--d:0.2s">the pulse is syncing with you</p>
          <p class="line muted" style="--d:1.2s">you can join it, or keep your edge</p>
          <div class="choice-row" style="--d:2.9s">
            <button
              class="choice"
              on:pointerenter={() => noteChoiceHoverEnter('decision2', 'sync')}
              on:pointerleave={() => noteChoiceHoverLeave('decision2', 'sync')}
              on:pointermove={() => noteChoiceHoverMove('decision2', 'sync')}
              on:click={() => pickD2('sync')}
            >
              sync with it
            </button>
            <span class="choice-div" aria-hidden="true"></span>
            <button
              class="choice"
              on:pointerenter={() => noteChoiceHoverEnter('decision2', 'resist')}
              on:pointerleave={() => noteChoiceHoverLeave('decision2', 'resist')}
              on:pointermove={() => noteChoiceHoverMove('decision2', 'resist')}
              on:click={() => pickD2('resist')}
            >
              hold your edge
            </button>
          </div>
        {/if}
      </div>
    {/if}

    {#if state === 'decision3'}
      <div class="screen" transition:fade={{ duration: 520 }}>
        <p class="line" style="--d:0.2s">it shifts as you shift</p>
        <p class="line muted" style="--d:1.28s">this next move is yours</p>
        <div class="choice-row" style="--d:3.1s">
          <button
            class="choice"
            on:pointerenter={() => noteChoiceHoverEnter('decision3', 'merge')}
            on:pointerleave={() => noteChoiceHoverLeave('decision3', 'merge')}
            on:pointermove={() => noteChoiceHoverMove('decision3', 'merge')}
            on:click={() => pickD3('merge')}
          >
            step inside
          </button>
          <span class="choice-div" aria-hidden="true"></span>
          <button
            class="choice"
            on:pointerenter={() => noteChoiceHoverEnter('decision3', 'observe')}
            on:pointerleave={() => noteChoiceHoverLeave('decision3', 'observe')}
            on:pointermove={() => noteChoiceHoverMove('decision3', 'observe')}
            on:click={() => pickD3('observe')}
          >
            stay and witness
          </button>
        </div>
      </div>
    {/if}

    {#if state === 'silence'}
      <div class="screen silence-screen" transition:fade={{ duration: 500 }}>
        <div class="silence-dot" aria-hidden="true"></div>
      </div>
    {/if}

    {#if state === 'reveal' || state === 'mapping'}
      <div
        class="screen reveal-screen"
        transition:fade={{ duration: 980 }}
      >
        <div class="reveal-layout">
          <section class="reveal-panel" aria-label="node and interpretation panel">
            <div class="reveal-panel-grid">
              <div class="node-column">
                <div class="node-block" class:mapping-muted={state === 'mapping'}>
                  <div class="node-line section" style="--d:0.4s">⊕ origin: onboarding.adventure.v1</div>
                  <div class="node-line" style="--d:0.72s">mode: implicit</div>
                  <div class="node-line" style="--d:1.02s">conf: 0.82</div>
                  <div class="node-gap" aria-hidden="true"></div>

                  <div class="node-line section" style="--d:1.9s">⦿ envelope</div>
                  <div class="node-line" style="--d:2.2s">grounding: {avec.stability}</div>
                  <div class="node-line" style="--d:2.5s">wear: {avec.friction}</div>
                  <div class="node-line" style="--d:2.8s">clarity: {avec.logic}</div>
                  <div class="node-line" style="--d:3.1s">self-trust: {avec.autonomy}</div>
                  <div class="node-line" style="--d:3.4s">psi: {avec.psi}</div>
                  <div class="node-gap" aria-hidden="true"></div>

                  <div class="node-line section" style="--d:4.35s">◈ trace</div>
                  <div class="node-line" style="--d:4.65s">d1: {d1 ?? '—'}</div>
                  <div class="node-line" style="--d:4.95s">name: {resolvedName}</div>
                  <div class="node-line" style="--d:5.25s">d2: {d2 ?? '—'}</div>
                  <div class="node-line" style="--d:5.55s">d3: {d3 ?? '—'}</div>
                  <div class="node-gap" aria-hidden="true"></div>

                  <div class="node-line section accent" style="--d:6.2s">⊛ attractor: {attractor()}</div>
                </div>
              </div>

              <aside class="effects-panel" class:ready={state === 'mapping'}>
                <p class="effects-kicker">how your choices moved the field</p>
                <div class="effects-list">
                  {#each decisionEffects as effect, i}
                    <div
                      class="effect-line"
                      class:active={state === 'mapping' && i < mappingStep}
                      class:preview={state === 'reveal' || (state === 'mapping' && i >= mappingStep)}
                      style="--d:{i * 0.08}s"
                    >
                      <span class="effect-index">{i + 1}</span>
                      <div class="effect-copy">
                        <p class="effect-choice">{effect.choice}</p>
                        <p class="effect-impact">{effect.impact}</p>
                      </div>
                    </div>
                  {/each}
                </div>

                {#if state === 'reveal'}
                  <button
                    class="reveal-continue panel-continue"
                    class:visible={revealHintVisible}
                    on:click|stopPropagation={handleRevealContinue}
                    disabled={!revealCanAdvance}
                  >
                    reveal the reading
                  </button>
                {/if}

                {#if state === 'mapping' && mappingStep >= mappingLines.length}
                  <p class="map-summary" transition:fade={{ duration: 440 }}>this is what resonated</p>

                  <div class="explain-stack" transition:fade={{ duration: 520 }}>
                    <article class="explain-card">
                      <p class="explain-title">what shifted</p>
                      <p class="explain-body">{explanationWhatHappened}</p>
                    </article>

                    <article class="explain-card">
                      <p class="explain-title">why the name matters</p>
                      <p class="explain-body">{explanationName}</p>
                    </article>

                    <article class="explain-card">
                      <p class="explain-title">what the field noticed</p>
                      <p class="explain-body">{explanationNotice}</p>
                    </article>

                    <article class="explain-card">
                      <p class="explain-title">where this lives in resonantia</p>
                      <p class="explain-body">{explanationRelation}</p>
                    </article>

                    <article class="explain-card metrics-card">
                      <p class="explain-title">behavioral weighting</p>
                      <p class="metric-intro">these numbers are not just stats. they are the trace of how your attention moved.</p>
                      <div class="metric-reading-stack">
                        <p class="metric-reading">{hoverStory}</p>
                        <p class="metric-reading">{hesitationStory}</p>
                        <p class="metric-reading">{explorationStory}</p>
                        <p class="metric-reading">{engagementStory}</p>
                        <p class="metric-reading">{weightingStory}</p>
                      </div>
                      <p class="metric-raw-title">raw trace</p>
                      <div class="metrics-grid">
                        <p class="metric-line"><span>glow hover</span><strong>{formatDuration(d1HoverLightMs)}</strong></p>
                        <p class="metric-line"><span>pulse hover</span><strong>{formatDuration(d1HoverSoundMs)}</strong></p>
                        <p class="metric-line"><span>decision styles</span><strong>{stageStyleSummary}</strong></p>
                        <p class="metric-line"><span>hesitation</span><strong>{signalMetrics.hesitationScore.toFixed(2)}</strong></p>
                        <p class="metric-line"><span>exploration</span><strong>{signalMetrics.explorationScore.toFixed(2)}</strong></p>
                        <p class="metric-line"><span>engagement</span><strong>{signalMetrics.engagementScore.toFixed(2)}</strong></p>
                        <p class="metric-line"><span>weight shifts</span><strong>{shiftSummary}</strong></p>
                        <p class="metric-line"><span>journey time</span><strong>{formatDuration(signalMetrics.totalDurationMs)}</strong></p>
                      </div>
                      <p class="metric-note">these traces shape grounding, wear, clarity, and self-trust before the node is planted.</p>
                    </article>
                  </div>

                  <button
                    class="mapping-continue"
                    on:click={advanceFromMapping}
                    disabled={!mappingCanAdvance}
                  >
                    carry this into memory
                  </button>
                {/if}
              </aside>
            </div>
          </section>
        </div>
      </div>
    {/if}

    {#if state === 'cta'}
      <div class="screen cta-screen" transition:fade={{ duration: 700 }}>
        <p class="cta-attractor">⊛ {attractor()}</p>
        <p class="cta-label">plant this as your first node in the constellation</p>
        <button
          class="cta-btn"
          on:pointerenter={() => noteChoiceHoverEnter('cta', 'plant')}
          on:pointerleave={() => noteChoiceHoverLeave('cta', 'plant')}
          on:pointermove={() => noteChoiceHoverMove('cta', 'plant')}
          on:click={plantMemory}
        >
          plant first memory
        </button>
        <button
          class="cta-skip"
          on:pointerenter={() => noteChoiceHoverEnter('cta', 'skip')}
          on:pointerleave={() => noteChoiceHoverLeave('cta', 'skip')}
          on:pointermove={() => noteChoiceHoverMove('cta', 'skip')}
          on:click={handleSkip}
        >
          not now
        </button>
      </div>
    {/if}

    {#if state === 'confirmed'}
      <div class="screen confirm-screen" transition:fade={{ duration: 450 }}>
        <div class="confirm-node">⊛ {attractor()}</div>
        <p class="confirm-word">anchored</p>
      </div>
    {/if}

    {#if state !== 'silence' && state !== 'confirmed'}
      <button class="exit-hatch" on:click={handleSkip} tabindex="-1">leave</button>
    {/if}
  </div>
{/if}

<style>
  .adv {
    --tone-a: rgba(103, 117, 175, 0.16);
    --tone-b: rgba(56, 78, 128, 0.14);
    --tone-c: rgba(15, 23, 44, 0.7);
    position: fixed;
    inset: 0;
    z-index: 9999;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: 'Departure Mono', 'Courier New', monospace;
    color: rgba(255, 255, 255, 0.85);
    overflow: hidden;
    isolation: isolate;
    background:
      radial-gradient(120% 95% at 88% 12%, var(--tone-a) 0%, transparent 58%),
      radial-gradient(95% 95% at 15% 78%, var(--tone-b) 0%, transparent 63%),
      linear-gradient(170deg, #02030b 0%, #040916 45%, #020409 100%);
    transition: background 1.45s ease;
  }

  .adv.theme-light {
    --tone-a: rgba(210, 184, 129, 0.2);
    --tone-b: rgba(142, 122, 86, 0.18);
    --tone-c: rgba(48, 34, 16, 0.72);
  }

  .adv.theme-sound {
    --tone-a: rgba(79, 187, 219, 0.2);
    --tone-b: rgba(47, 116, 152, 0.22);
    --tone-c: rgba(7, 39, 58, 0.74);
  }

  .ambient-nebula {
    position: absolute;
    inset: -24%;
    z-index: 0;
    pointer-events: none;
    background:
      radial-gradient(ellipse at 30% 26%, color-mix(in srgb, var(--tone-a) 72%, transparent) 0%, transparent 58%),
      radial-gradient(ellipse at 70% 66%, color-mix(in srgb, var(--tone-b) 66%, transparent) 0%, transparent 62%);
    filter: blur(18px) saturate(1.15);
    animation: nebula-drift 18s ease-in-out infinite alternate;
  }

  .vignette {
    position: absolute;
    inset: 0;
    z-index: 1;
    pointer-events: none;
    background:
      radial-gradient(ellipse at center, transparent 37%, rgba(0, 0, 0, 0.26) 72%, rgba(0, 0, 0, 0.54) 100%),
      linear-gradient(180deg, rgba(0, 0, 0, 0.22) 0%, transparent 22%, transparent 72%, rgba(0, 0, 0, 0.3) 100%);
  }

  .particles {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 0;
  }

  .star {
    --drift: 6px;
    position: absolute;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.56);
    animation: star-breathe linear infinite;
  }

  .star[data-layer='0'] {
    --drift: 3px;
    opacity: 0.46;
  }

  .star[data-layer='1'] {
    --drift: 6px;
    opacity: 0.58;
  }

  .star[data-layer='2'] {
    --drift: 10px;
    opacity: 0.72;
  }

  @keyframes star-breathe {
    0% {
      opacity: 0.14;
      transform: translate3d(calc(var(--drift) * -0.35), calc(var(--drift) * 0.2), 0) scale(0.95);
    }
    45% {
      opacity: 0.9;
      transform: translate3d(calc(var(--drift) * 0.4), calc(var(--drift) * -0.25), 0) scale(1.58);
    }
    100% {
      opacity: 0.2;
      transform: translate3d(calc(var(--drift) * -0.2), calc(var(--drift) * 0.15), 0) scale(1.02);
    }
  }

  .screen {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    gap: 0.35rem;
  }

  .line {
    margin: 0;
    max-width: min(88vw, 50rem);
    text-align: center;
    font-size: clamp(0.95rem, 2.45vw, 1.24rem);
    line-height: 1.65;
    letter-spacing: 0.045em;
    color: rgba(255, 255, 255, 0.9);
    opacity: 0;
    animation: fade-up 0.95s cubic-bezier(0.19, 0.86, 0.25, 1) forwards;
    animation-delay: var(--d, 0s);
  }

  .line.muted {
    color: rgba(255, 255, 255, 0.48);
    font-size: clamp(0.82rem, 1.9vw, 0.98rem);
  }

  @keyframes fade-up {
    from {
      opacity: 0;
      transform: translate3d(0, 13px, 0);
      filter: blur(2px);
    }
    to {
      opacity: 1;
      transform: translate3d(0, 0, 0);
      filter: blur(0);
    }
  }

  .d1-screen {
    flex-direction: row;
    align-items: stretch;
    justify-content: stretch;
    padding: 0;
    gap: 0;
  }

  .zone {
    flex: 1;
    height: 100%;
    border: none;
    background: transparent;
    cursor: pointer;
    position: relative;
    overflow: hidden;
    transition: filter 0.42s ease, background 0.5s ease;
  }

  .zone:focus-visible {
    outline: 1px solid rgba(255, 255, 255, 0.26);
    outline-offset: -4px;
  }

  .zone-light {
    background: radial-gradient(
      140% 120% at var(--mx, 68%) var(--my, 50%),
      rgba(226, 199, 149, 0.08) 0%,
      transparent 58%
    );
  }

  .light-glow {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(circle at var(--mx, 68%) var(--my, 50%), rgba(226, 198, 141, 0.2) 0%, rgba(226, 198, 141, 0.06) 22%, transparent 58%),
      radial-gradient(ellipse at 72% 50%, rgba(244, 224, 176, 0.09) 0%, transparent 65%);
    filter: blur(1px);
    animation: light-breathe 4.8s ease-in-out infinite;
    transition: background 0.32s ease;
  }

  .light-flare {
    position: absolute;
    inset: 0;
    background: linear-gradient(104deg, transparent 0%, rgba(255, 248, 221, 0.08) 52%, transparent 100%);
    transform: translateX(-18%);
    opacity: 0.32;
    mix-blend-mode: screen;
    animation: light-flare 5.8s ease-in-out infinite;
  }

  .zone-light.zone-hovered {
    filter: brightness(1.15) saturate(1.08);
  }

  .zone-light.zone-hovered .light-glow {
    background:
      radial-gradient(circle at var(--mx, 68%) var(--my, 50%), rgba(238, 210, 150, 0.32) 0%, rgba(238, 210, 150, 0.1) 24%, transparent 55%),
      radial-gradient(ellipse at 72% 50%, rgba(255, 230, 175, 0.12) 0%, transparent 64%);
  }

  @keyframes light-breathe {
    0%,
    100% {
      opacity: 0.56;
      transform: scale(1);
    }
    50% {
      opacity: 1;
      transform: scale(1.05);
    }
  }

  @keyframes light-flare {
    0%,
    100% {
      transform: translateX(-18%);
      opacity: 0.2;
    }
    50% {
      transform: translateX(12%);
      opacity: 0.36;
    }
  }

  .zone-sound {
    border-left: 0.5px solid rgba(255, 255, 255, 0.07);
    background:
      radial-gradient(circle at var(--mx, 32%) var(--my, 50%), rgba(84, 195, 231, 0.09) 0%, transparent 58%),
      linear-gradient(180deg, rgba(42, 96, 130, 0.05) 0%, transparent 100%);
  }

  .zone-sound.zone-hovered {
    filter: brightness(1.12) saturate(1.14);
  }

  .pulse-ring {
    position: absolute;
    top: var(--my, 50%);
    left: var(--mx, 32%);
    width: 68px;
    height: 68px;
    border: 1px solid rgba(110, 201, 230, 0.22);
    border-radius: 50%;
    transform: translate(-50%, -50%);
    transform-origin: center;
    pointer-events: none;
    animation: pulse-expand 4.2s ease-out infinite;
    transition: border-color 0.3s ease;
  }

  .zone-sound.zone-hovered .pulse-ring {
    border-color: rgba(111, 209, 240, 0.45);
    animation-duration: 2.6s;
  }

  @keyframes pulse-expand {
    0% {
      opacity: 0.95;
      transform: translate(-50%, -50%) scale(0.68);
    }
    62% {
      opacity: 0.25;
      transform: translate(-50%, -50%) scale(4.9);
    }
    100% {
      opacity: 0;
      transform: translate(-50%, -50%) scale(5.9);
    }
  }

  .d1-caption {
    position: absolute;
    bottom: 2.6rem;
    left: 0;
    right: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.28rem;
    pointer-events: none;
    z-index: 3;
    padding: 0 1rem;
  }

  .name-wrap,
  .choice-row {
    opacity: 0;
    animation: fade-up 0.8s cubic-bezier(0.19, 0.86, 0.25, 1) forwards;
    animation-delay: var(--d, 0s);
  }

  .name-wrap {
    display: flex;
    align-items: center;
    gap: 0.78rem;
    margin-top: 2.3rem;
  }

  .name-input {
    background: transparent;
    border: none;
    border-bottom: 1px solid rgba(255, 255, 255, 0.24);
    color: rgba(255, 255, 255, 0.92);
    font-family: inherit;
    font-size: 1.05rem;
    letter-spacing: 0.06em;
    padding: 0.2rem 0.15rem 0.3rem;
    width: 220px;
    text-align: center;
    outline: none;
    caret-color: rgba(255, 255, 255, 0.56);
    transition: border-color 0.3s ease;
  }

  .name-input::placeholder {
    color: rgba(255, 255, 255, 0.16);
  }

  .name-input:focus {
    border-bottom-color: rgba(255, 255, 255, 0.62);
  }

  .name-confirm {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.46);
    font-family: inherit;
    font-size: 1.08rem;
    cursor: pointer;
    padding: 0;
    line-height: 1;
    transition: color 0.2s ease;
  }

  .name-confirm:hover {
    color: rgba(255, 255, 255, 0.9);
  }

  .choice-row {
    display: flex;
    align-items: center;
    gap: 1.55rem;
    margin-top: 2.45rem;
  }

  .choice {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.4);
    font-family: inherit;
    font-size: 0.88rem;
    letter-spacing: 0.08em;
    text-transform: lowercase;
    cursor: pointer;
    padding: 0.35rem 0.5rem;
    position: relative;
    transition: color 0.24s ease;
  }

  .choice::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 0.5px;
    background: rgba(255, 255, 255, 0.28);
    transform: scaleX(0);
    transition: transform 0.24s ease;
  }

  .choice:hover {
    color: rgba(255, 255, 255, 0.9);
  }

  .choice:hover::after {
    transform: scaleX(1);
  }

  .choice-div {
    display: block;
    width: 0.5px;
    height: 1em;
    background: rgba(255, 255, 255, 0.12);
    flex-shrink: 0;
  }

  .silence-screen {
    background: radial-gradient(circle at center, rgba(255, 255, 255, 0.02) 0%, transparent 48%);
  }

  .silence-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.56);
    box-shadow: 0 0 0 0 rgba(255, 255, 255, 0.2);
    animation: silence-breathe 2.2s ease-in-out infinite;
  }

  @keyframes silence-breathe {
    0%,
    100% {
      transform: scale(0.85);
      opacity: 0.4;
      box-shadow: 0 0 0 0 rgba(255, 255, 255, 0.12);
    }
    50% {
      transform: scale(1.08);
      opacity: 0.92;
      box-shadow: 0 0 0 14px rgba(255, 255, 255, 0);
    }
  }

  .reveal-screen {
    justify-content: flex-start;
    align-items: center;
    gap: 1.1rem;
    padding-top: clamp(1.7rem, 6.4vh, 4rem);
    padding-bottom: 2.3rem;
    overflow-y: auto;
  }

  .reveal-layout {
    width: min(82rem, 96vw);
    display: flex;
    justify-content: center;
  }

  .reveal-panel {
    width: 100%;
    border: 0.5px solid rgba(255, 255, 255, 0.16);
    border-radius: 10px;
    background: linear-gradient(150deg, rgba(255, 255, 255, 0.032), rgba(255, 255, 255, 0.013));
    box-shadow: 0 18px 46px rgba(0, 0, 0, 0.27);
    padding: clamp(0.95rem, 1.9vw, 1.35rem);
    backdrop-filter: blur(2px);
  }

  .reveal-panel-grid {
    display: flex;
    align-items: flex-start;
    gap: clamp(0.95rem, 2vw, 1.65rem);
  }

  .node-column {
    flex: 1 1 55%;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 1.05rem;
    min-width: 0;
  }

  .node-block {
    width: 100%;
    padding: 0.22rem 0.24rem;
    border: none;
    border-radius: 0;
    background: transparent;
    backdrop-filter: none;
    box-shadow: none;
    transition: opacity 0.46s ease, filter 0.46s ease, transform 0.46s ease;
  }

  .node-block.mapping-muted {
    opacity: 0.82;
    filter: saturate(0.95);
    transform: translateY(-1px);
  }

  .node-line {
    margin: 0;
    font-size: clamp(0.69rem, 1.34vw, 0.86rem);
    line-height: 1.62;
    letter-spacing: 0.035em;
    color: rgba(255, 255, 255, 0.5);
    opacity: 0;
    transform: translate3d(0, 10px, 0);
    animation: fade-up 0.78s cubic-bezier(0.19, 0.86, 0.25, 1) forwards;
    animation-delay: var(--d, 0s);
  }

  .node-line.section {
    color: rgba(255, 255, 255, 0.72);
  }

  .node-line.accent {
    color: rgba(255, 255, 255, 0.84);
  }

  .node-gap {
    height: 0.7rem;
  }

  .effects-panel {
    flex: 1 1 45%;
    border-left: 0.5px solid rgba(255, 255, 255, 0.14);
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    padding: 0.15rem 0.25rem 0.15rem 1rem;
    min-width: 0;
  }

  .effects-kicker {
    margin: 0;
    font-size: 0.73rem;
    letter-spacing: 0.08em;
    text-transform: lowercase;
    color: rgba(255, 255, 255, 0.62);
  }

  .effects-list {
    margin-top: 0.7rem;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .effect-line {
    display: grid;
    grid-template-columns: 1.3rem minmax(0, 1fr);
    align-items: start;
    gap: 0.55rem;
    opacity: 0.54;
    transform: translate3d(0, 8px, 0);
    transition: opacity 0.24s ease, transform 0.24s ease, background 0.24s ease;
    padding: 0.35rem 0.4rem;
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.01);
  }

  .effect-line.active {
    opacity: 0.96;
    transform: translate3d(0, 0, 0);
    background: rgba(255, 255, 255, 0.07);
  }

  .effect-line.preview {
    opacity: 0.72;
    transform: translate3d(0, 0, 0);
    background: rgba(255, 255, 255, 0.028);
  }

  .effect-index {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.2rem;
    height: 1.2rem;
    border: 0.5px solid rgba(255, 255, 255, 0.18);
    border-radius: 999px;
    font-size: 0.67rem;
    color: rgba(255, 255, 255, 0.62);
  }

  .effect-copy {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
  }

  .effect-choice,
  .effect-impact {
    margin: 0;
  }

  .effect-choice {
    font-size: 0.75rem;
    color: rgba(255, 255, 255, 0.86);
    letter-spacing: 0.035em;
  }

  .effect-impact {
    font-size: 0.72rem;
    color: rgba(255, 255, 255, 0.58);
    letter-spacing: 0.02em;
    line-height: 1.45;
  }

  .panel-continue {
    margin-top: 0.78rem;
  }

  .reveal-continue {
    margin: 0;
    font-size: 0.76rem;
    color: rgba(255, 255, 255, 0.3);
    letter-spacing: 0.08em;
    text-transform: lowercase;
    border: none;
    background: transparent;
    font-family: inherit;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.42s ease;
    animation: hint-pulse 2.3s ease-in-out infinite;
  }

  .reveal-continue.visible {
    opacity: 1;
  }

  .reveal-continue:disabled {
    cursor: default;
    opacity: 0;
  }

  @keyframes hint-pulse {
    0%,
    100% {
      transform: translateY(0);
      color: rgba(255, 255, 255, 0.32);
    }
    50% {
      transform: translateY(-2px);
      color: rgba(255, 255, 255, 0.55);
    }
  }

  .explain-stack {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    margin-top: 0.72rem;
  }

  .explain-card {
    border: 0.5px solid rgba(255, 255, 255, 0.09);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.018);
    padding: 0.56rem 0.62rem;
  }

  .explain-title,
  .explain-body {
    margin: 0;
  }

  .explain-title {
    font-size: 0.7rem;
    letter-spacing: 0.07em;
    text-transform: lowercase;
    color: rgba(255, 255, 255, 0.69);
  }

  .explain-body {
    margin-top: 0.18rem;
    font-size: 0.72rem;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.5);
  }

  .metrics-card {
    border-color: rgba(255, 255, 255, 0.14);
    background: rgba(255, 255, 255, 0.026);
  }

  .metric-intro {
    margin: 0.28rem 0 0;
    font-size: 0.7rem;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.68);
  }

  .metric-reading-stack {
    margin-top: 0.42rem;
    display: flex;
    flex-direction: column;
    gap: 0.24rem;
  }

  .metric-reading {
    margin: 0;
    font-size: 0.68rem;
    line-height: 1.45;
    color: rgba(255, 255, 255, 0.56);
  }

  .metric-raw-title {
    margin: 0.48rem 0 0;
    font-size: 0.66rem;
    text-transform: lowercase;
    letter-spacing: 0.07em;
    color: rgba(255, 255, 255, 0.47);
  }

  .metrics-grid {
    margin-top: 0.28rem;
    display: grid;
    gap: 0.26rem;
  }

  .metric-line {
    margin: 0;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 0.7rem;
    font-size: 0.68rem;
    color: rgba(255, 255, 255, 0.56);
    letter-spacing: 0.02em;
  }

  .metric-line strong {
    color: rgba(255, 255, 255, 0.78);
    font-weight: 500;
    text-align: right;
  }

  .metric-note {
    margin: 0.38rem 0 0;
    font-size: 0.67rem;
    line-height: 1.45;
    color: rgba(255, 255, 255, 0.46);
  }

  .map-summary {
    margin: 0.8rem 0 0;
    font-size: 0.82rem;
    color: rgba(255, 255, 255, 0.72);
    letter-spacing: 0.08em;
    text-transform: lowercase;
  }

  .mapping-continue {
    margin-top: 0.8rem;
    width: 100%;
    padding: 0.5rem 0.72rem;
    border: 0.5px solid rgba(255, 255, 255, 0.26);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.03);
    color: rgba(255, 255, 255, 0.86);
    font-family: inherit;
    font-size: 0.74rem;
    letter-spacing: 0.06em;
    text-transform: lowercase;
    cursor: pointer;
    transition: border-color 0.22s ease, background 0.22s ease, color 0.22s ease;
  }

  .mapping-continue:hover {
    border-color: rgba(255, 255, 255, 0.62);
    background: rgba(255, 255, 255, 0.06);
    color: rgba(255, 255, 255, 0.97);
  }

  .mapping-continue:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .cta-screen {
    gap: 1.05rem;
  }

  .cta-attractor,
  .cta-label,
  .cta-btn,
  .cta-skip {
    opacity: 0;
    animation: fade-up 0.82s cubic-bezier(0.19, 0.86, 0.25, 1) forwards;
    animation-fill-mode: both;
  }

  .cta-attractor {
    margin: 0;
    font-size: clamp(0.95rem, 2.8vw, 1.22rem);
    color: rgba(255, 255, 255, 0.82);
    letter-spacing: 0.06em;
    animation-delay: 0.14s;
  }

  .cta-label {
    margin: 0;
    font-size: 0.88rem;
    color: rgba(255, 255, 255, 0.5);
    letter-spacing: 0.06em;
    text-align: center;
    animation-delay: 0.4s;
  }

  .cta-btn {
    margin-top: 0.25rem;
    background: transparent;
    border: 0.5px solid rgba(255, 255, 255, 0.32);
    border-radius: 2px;
    color: rgba(255, 255, 255, 0.86);
    font-family: inherit;
    font-size: 0.86rem;
    letter-spacing: 0.08em;
    text-transform: lowercase;
    padding: 0.54rem 1.58rem;
    cursor: pointer;
    transition: border-color 0.22s ease, color 0.22s ease, background 0.22s ease;
    animation-delay: 0.66s;
  }

  .cta-btn:hover {
    border-color: rgba(255, 255, 255, 0.74);
    color: rgba(255, 255, 255, 1);
    background: rgba(255, 255, 255, 0.05);
  }

  .cta-skip {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.22);
    font-family: inherit;
    font-size: 0.74rem;
    letter-spacing: 0.06em;
    cursor: pointer;
    padding: 0.2rem;
    transition: color 0.2s ease;
    animation-delay: 0.88s;
  }

  .cta-skip:hover {
    color: rgba(255, 255, 255, 0.52);
  }

  .confirm-screen {
    gap: 0.3rem;
  }

  .confirm-node {
    margin: 0;
    font-size: clamp(0.9rem, 2.5vw, 1.12rem);
    color: rgba(255, 255, 255, 0.72);
    letter-spacing: 0.055em;
    animation: collapse-node 0.9s cubic-bezier(0.2, 0.86, 0.28, 1) forwards;
  }

  .confirm-word {
    margin: 0;
    font-size: clamp(0.84rem, 1.9vw, 0.94rem);
    color: rgba(255, 255, 255, 0.46);
    letter-spacing: 0.08em;
    text-transform: lowercase;
    opacity: 0;
    animation: fade-up 0.75s ease forwards;
    animation-delay: 0.56s;
  }

  @keyframes collapse-node {
    0% {
      opacity: 0.8;
      transform: scale(1);
      filter: blur(0);
    }
    72% {
      opacity: 0.46;
      transform: scale(0.34);
      filter: blur(0.8px);
    }
    100% {
      opacity: 0;
      transform: scale(0.08);
      filter: blur(2.4px);
    }
  }

  .exit-hatch {
    position: absolute;
    top: 1.2rem;
    right: 1.5rem;
    z-index: 10;
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.14);
    font-family: inherit;
    font-size: 0.7rem;
    letter-spacing: 0.06em;
    cursor: pointer;
    padding: 0.25rem;
    transition: color 0.22s ease;
  }

  .exit-hatch:hover {
    color: rgba(255, 255, 255, 0.46);
  }

  @keyframes nebula-drift {
    0% {
      transform: translate3d(-2%, -1%, 0) scale(1.03);
    }
    100% {
      transform: translate3d(3%, 2%, 0) scale(1.09);
    }
  }

  @media (max-width: 740px) {
    .d1-caption {
      bottom: 2rem;
    }

    .choice-row {
      gap: 1.2rem;
      margin-top: 2rem;
    }

    .reveal-layout {
      width: min(37rem, 95vw);
    }

    .reveal-panel {
      padding: 0.85rem 0.8rem;
    }

    .reveal-panel-grid {
      flex-direction: column;
      gap: 0.72rem;
    }

    .node-column {
      width: 100%;
    }

    .node-block {
      width: 100%;
      padding: 0;
    }

    .effects-panel {
      width: 100%;
      border-left: none;
      border-top: 0.5px solid rgba(255, 255, 255, 0.14);
      padding: 0.72rem 0 0;
    }
  }
</style>