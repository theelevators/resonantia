// See https://kit.svelte.dev/docs/types#app

declare module "/graph.js" {
  export function renderSessionGraph(
    elementId: string,
    payload: unknown,
    callbacks?: {
      onGraphSessionSelected?: (sessionId: string) => void;
      onGraphNodeSelected?: (sessionId: string, nodeIndex: number) => void;
    }
  ): Promise<void>;
  export function destroySessionGraph(elementId: string): void;
}

declare module "/theme.js" {
  export function applyTheme(themeKey: string, persist?: boolean): void;
  export function restoreTheme(fallbackKey: string): string;
}

declare module "/swipe.js" {
  export function attach(
    elementId: string,
    handlers: {
      onSwipeLeft?: () => void;
      onSwipeRight?: () => void;
      onSwipeUp?: () => void;
      onSwipeDown?: () => void;
    }
  ): void;
  export function detach(elementId: string): void;
}
