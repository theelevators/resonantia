---

# 🎮 ONBOARDING: FINAL SCRIPT + UI FLOW

## GLOBAL RULES (do not break these)

* max **1–2 sentences per screen**
* no paragraphs
* no explanations
* ambient audio carries mood
* transitions do the storytelling

---

# 🌌 STATE 0 — ENTRY

### UI:

* dark screen
* subtle motion (barely visible particles / waves)
* no buttons yet

### Text (fade in slowly):

> you arrive somewhere you don’t remember choosing

(beat — 800ms)

> it’s quiet. too quiet.

---

### Transition:

* faint **light glow** appears on one side
* subtle **pulse sound** on the other

---

# ⚡ STATE 1 — DECISION 1

### UI:

* two interactable regions (not buttons)

  * left: soft light
  * right: pulsing distortion

hover = slight amplification (NOT labels)

---

### Text:

> something is calling you

(beat)

> do you move toward it…
> or listen?

---

### Interaction:

* click light → PATH_LIGHT
* click pulse → PATH_SOUND

---

### Micro-feedback:

* selection causes immediate environmental shift
* no “you chose X” text

---

# 🧩 STATE 2A — LIGHT PATH

### Transition:

* environment sharpens
* geometry forms (clean, structured)

---

### Text:

> it’s forming… but not complete

(beat)

> it’s waiting

---

### UI:

* incomplete geometric structure
* subtle “gap” visible

---

### Decision:

> do you finish it…
> or leave it as it is?

---

### Interaction:

* finish → STRUCTURE_COMPLETE
* leave → STRUCTURE_OPEN

---

---

# 🌊 STATE 2B — SOUND PATH

### Transition:

* environment becomes fluid
* pulse syncs with cursor movement

---

### Text:

> it’s not out there anymore

(beat)

> it’s responding to you

---

### UI:

* rhythmic distortion tied to cursor

---

### Decision:

> do you sync with it…
> or pull away?

---

### Interaction:

* sync → FLOW_SYNC
* pull away → FLOW_RESIST

---

---

# 🧬 STATE 3 — CONVERGENCE (ALL PATHS)

### Transition:

* environment collapses inward
* previous choice effects blend into one form

---

### Text:

> it changes when you do

(beat)

> it always did

---

### UI:

* object/entity reflecting prior choices

  * geometric if structured
  * fluid if autonomous
  * hybrid if mixed

---

### Final Decision:

> do you step into it…
> or stay where you are?

---

### Interaction:

* step in → MERGE
* stay → OBSERVE

---

---

# ⏳ STATE 4 — SILENCE (CRITICAL)

### UI:

* everything freezes

### NO TEXT

### Duration:

* ~1000–1200ms

---

👉 this is where meaning forms
do NOT skip this

---

---

# ⏣ STATE 5 — NODE REVEAL (RAW)

### UI:

* STTP node fades in
* monospaced / structured / artifact-like
* looks *real*, not decorative

---

### Content (user structure example):

```
⊕ origin: onboarding.adventure.v1
   mode: implicit
   confidence: 0.82

⦿ envelope:
   stability: 0.78
   friction: 0.34
   logic: 0.71
   autonomy: 0.69
   psi: 2.84

◈ trace:
   d1: light
   d2: complete
   d3: step_in

⊛ attractor:
   structured openness
```

---

### IMPORTANT:

No explanation yet.
Let them stare for ~1.5–2s.

---

---

# 🔍 STATE 6 — MAPPING (ANIMATION)

### UI:

animate mappings line by line:

* “moved toward the light” → stability + logic ↑
* “finished the structure” → stability ↑
* “stepped in” → autonomy ↑

Each line appears with subtle motion.

---

### Then:

fade everything except summary

---

### Text (first human sentence):

> this is what we heard

---

---

# 🌱 STATE 7 — CTA

### UI:

* node remains visible (slightly dimmed)
* one button only

---

### Text:

> plant this as your first memory

---

### Button:

> [ plant memory ]

---

### Secondary (very subtle):

> skip for now

---

---

# 🧠 STATE 8 — CONFIRMATION (POST-CTA)

(if clicked)

### UI:

* node collapses into a point
* flows into “memory space” (your main UI)

---

### Text:

> saved

(then transition immediately into app)

---

---

# ⚙️ IMPLEMENTATION NOTES (IMPORTANT)

## 1. No visible steps

User should NOT feel:

* “step 1 / 2 / 3”

This must feel continuous.

---

## 2. Cursor = part of system

Especially in SOUND path:

* movement = interaction
* creates subconscious agency

---

## 3. No back buttons

Forward only.

Reduces friction + increases commitment.

---

## 4. Escape hatch

Top corner, faint:

> exit

(for non-engagers)

---

---

# 🔥 FINAL EXPERIENCE FLOW (CONDENSED)

```
arrival →
decision →
decision →
decision →
silence →
node →
realization →
action
```

---

# 🧨 WHY THIS VERSION WORKS

* excited user → feels depth + payoff
* skeptic → gets surprised at convergence
* non-engager → finishes in ~20–30 seconds

---
