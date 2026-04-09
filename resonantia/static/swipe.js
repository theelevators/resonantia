const swipeRegistry = new Map();

export function attach(elementId, handlers) {
  const el = document.getElementById(elementId);
  if (!el) return;

  let startX = 0;
  let startY = 0;
  let dx = 0;
  let dy = 0;
  let active = false;
  let pointerType = "mouse";
  const THRESHOLD = 72;

  function hint(dir) {
    return el.querySelector(`.hint-${dir}`);
  }

  function clearHints() {
    ["left", "right", "up", "down"].forEach((d) => {
      const h = hint(d);
      if (h) h.style.opacity = "0";
    });
  }

  function springBack() {
    el.style.transition = "transform 0.32s cubic-bezier(0.34, 1.56, 0.64, 1)";
    el.style.transform = "";
    el.style.opacity = "";
    clearHints();
  }

  function onDown(e) {
    if (e.target.closest("button, a, input, textarea, select")) return;
    startX = e.clientX;
    startY = e.clientY;
    dx = 0;
    dy = 0;
    active = true;
    pointerType = e.pointerType || "mouse";
    el.setPointerCapture(e.pointerId);
    el.style.transition = "none";
  }

  function onMove(e) {
    if (!active) return;
    dx = e.clientX - startX;
    dy = e.clientY - startY;

    const absDx = Math.abs(dx);
    const absDy = Math.abs(dy);
    const isHoriz = absDx >= absDy;

    const rot = isHoriz ? dx * 0.045 : 0;
    const tx = isHoriz ? dx : dx * 0.1;
    const ty = isHoriz ? dy * 0.08 : dy * 0.5;
    el.style.transform = `translate(${tx}px, ${ty}px) rotate(${rot}deg)`;

    const prog = (amt) => Math.max(0, Math.min(1, (amt - 20) / (THRESHOLD - 20)));

    if (isHoriz) {
      const activeDir = dx < 0 ? "left" : "right";
      const inactive = dx < 0 ? "right" : "left";
      const h = hint(activeDir);
      if (h) h.style.opacity = `${prog(absDx)}`;
      const h2 = hint(inactive);
      if (h2) h2.style.opacity = "0";
      const hu = hint("up");
      if (hu) hu.style.opacity = "0";
      const hd = hint("down");
      if (hd) hd.style.opacity = "0";
    } else {
      const activeDir = dy < 0 ? "up" : "down";
      const inactive = dy < 0 ? "down" : "up";
      const h = hint(activeDir);
      if (h) h.style.opacity = `${prog(absDy)}`;
      const h2 = hint(inactive);
      if (h2) h2.style.opacity = "0";
      const hl = hint("left");
      if (hl) hl.style.opacity = "0";
      const hr = hint("right");
      if (hr) hr.style.opacity = "0";
    }
  }

  function onUp() {
    if (!active) return;
    active = false;

    const absDx = Math.abs(dx);
    const absDy = Math.abs(dy);

    if (absDx > absDy && absDx > THRESHOLD) {
      const goLeft = dx < 0;
      el.style.transition = "transform 0.25s ease-in, opacity 0.25s ease-in";
      el.style.transform = `translateX(${goLeft ? -140 : 140}%) rotate(${goLeft ? -14 : 14}deg)`;
      el.style.opacity = "0";
      setTimeout(() => {
        clearHints();
        if (goLeft) {
          handlers?.onSwipeLeft?.();
        } else {
          handlers?.onSwipeRight?.();
        }
      }, 230);
    } else if (absDy > absDx && absDy > THRESHOLD) {
      springBack();
      if (pointerType !== "touch") {
        if (dy < 0) {
          handlers?.onSwipeUp?.();
        } else {
          handlers?.onSwipeDown?.();
        }
      }
    } else {
      springBack();
    }

    dx = 0;
    dy = 0;
  }

  function onCancel() {
    if (!active) return;
    active = false;
    springBack();
    dx = 0;
    dy = 0;
  }

  el.addEventListener("pointerdown", onDown);
  el.addEventListener("pointermove", onMove);
  el.addEventListener("pointerup", onUp);
  el.addEventListener("pointercancel", onCancel);

  swipeRegistry.set(elementId, { onDown, onMove, onUp, onCancel });
}

export function detach(elementId) {
  const el = document.getElementById(elementId);
  const handlers = swipeRegistry.get(elementId);
  if (!el || !handlers) return;

  el.removeEventListener("pointerdown", handlers.onDown);
  el.removeEventListener("pointermove", handlers.onMove);
  el.removeEventListener("pointerup", handlers.onUp);
  el.removeEventListener("pointercancel", handlers.onCancel);
  swipeRegistry.delete(elementId);
}
