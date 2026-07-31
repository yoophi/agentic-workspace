import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

import {
  adjustFontSizeStep,
  getAppearancePreferences,
  listenAppearancePreferences,
  setFontSizeStep,
} from "@/entities/appearance-preferences/api/appearance-preferences-repository";
import {
  DEFAULT_FONT_SIZE_STEP,
  normalizeFontSizeStep,
} from "@/entities/appearance-preferences/model/font-size-step";
import type {
  AppearancePreferences,
  FontSizeAdjustment,
  FontSizeStep,
} from "@/entities/appearance-preferences/model/types";
import { fontSizeAdjustmentForShortcut } from "@/features/font-size-adjustment/model/keyboard-shortcut";

type AppearancePreferencesContextValue = {
  fontSizeStep: FontSizeStep;
  isHydrated: boolean;
  error: string | null;
  setFontSizeStep: (step: FontSizeStep) => Promise<AppearancePreferences>;
  adjustFontSizeStep: (delta: FontSizeAdjustment) => Promise<AppearancePreferences>;
};

const unavailable = async (): Promise<AppearancePreferences> => {
  throw new Error("AppearancePreferencesProvider가 필요합니다.");
};

const AppearancePreferencesContext =
  createContext<AppearancePreferencesContextValue>({
    fontSizeStep: DEFAULT_FONT_SIZE_STEP,
    isHydrated: false,
    error: null,
    setFontSizeStep: unavailable,
    adjustFontSizeStep: unavailable,
  });

type DatasetTarget = { dataset: { fontSizeStep?: string } };

export function applyFontSizeStep(target: DatasetTarget, step: FontSizeStep) {
  const next = String(step);
  if (target.dataset.fontSizeStep === next) {
    return;
  }
  target.dataset.fontSizeStep = next;
}

export async function hydrateAppearancePreferences({
  listen,
  get,
  apply,
  ready,
}: {
  listen: (
    onChange: (preferences: AppearancePreferences) => void,
  ) => Promise<() => void>;
  get: () => Promise<AppearancePreferences>;
  apply: (preferences: AppearancePreferences) => void;
  ready: (error: string | null) => void;
}) {
  // An event that lands while `get` is in flight already carries the newest value,
  // so the snapshot must not overwrite it.
  let sawEvent = false;
  let listenError: string | null = null;
  let unlisten: () => void = () => undefined;
  try {
    unlisten = await listen((preferences) => {
      sawEvent = true;
      apply(preferences);
    });
  } catch (error) {
    listenError = String(error);
  }
  try {
    const preferences = await get();
    if (!sawEvent) {
      apply(preferences);
    }
    ready(listenError);
  } catch (error) {
    if (!sawEvent) {
      apply({ fontSizeStep: DEFAULT_FONT_SIZE_STEP });
    }
    ready(
      [listenError, String(error)].filter(Boolean).join("; "),
    );
  }
  return unlisten;
}

export function installFontSizeShortcut(
  target: Pick<Window, "addEventListener" | "removeEventListener">,
  adjust: (delta: FontSizeAdjustment) => Promise<unknown>,
  onError: (error: unknown) => void,
) {
  const onKeyDown = (event: KeyboardEvent) => {
    const delta = fontSizeAdjustmentForShortcut(event);
    if (delta === null) {
      return;
    }
    event.preventDefault();
    void adjust(delta).catch(onError);
  };
  target.addEventListener("keydown", onKeyDown, { capture: true });
  return () => target.removeEventListener("keydown", onKeyDown, { capture: true });
}

export function AppearancePreferencesProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [fontSizeStep, setStep] = useState<FontSizeStep>(DEFAULT_FONT_SIZE_STEP);
  const [isHydrated, setHydrated] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const apply = useCallback((preferences: AppearancePreferences) => {
    const canonical = normalizeFontSizeStep(preferences.fontSizeStep);
    applyFontSizeStep(document.documentElement, canonical);
    setStep((current) => (current === canonical ? current : canonical));
  }, []);

  const mutate = useCallback(
    async (call: () => Promise<AppearancePreferences>) => {
      setError(null);
      try {
        const preferences = await call();
        apply(preferences);
        return preferences;
      } catch (cause) {
        setError(String(cause));
        throw cause;
      }
    },
    [apply],
  );

  const setCanonicalStep = useCallback(
    (step: FontSizeStep) => mutate(() => setFontSizeStep(step)),
    [mutate],
  );

  const adjustCanonicalStep = useCallback(
    (delta: FontSizeAdjustment) => mutate(() => adjustFontSizeStep(delta)),
    [mutate],
  );

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void hydrateAppearancePreferences({
      listen: (onChange) =>
        listenAppearancePreferences((preferences) => {
          if (active) {
            onChange(preferences);
          }
        }),
      get: getAppearancePreferences,
      apply: (preferences) => {
        if (active) {
          apply(preferences);
        }
      },
      ready: (loadError) => {
        if (active) {
          setError(loadError);
          setHydrated(true);
        }
      },
    }).then((cleanup) => {
      if (active) {
        unlisten = cleanup;
      } else {
        cleanup();
      }
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [apply]);

  useEffect(
    () =>
      installFontSizeShortcut(window, adjustCanonicalStep, (cause) =>
        setError(String(cause)),
      ),
    [adjustCanonicalStep],
  );

  const value = useMemo(
    () => ({
      fontSizeStep,
      isHydrated,
      error,
      setFontSizeStep: setCanonicalStep,
      adjustFontSizeStep: adjustCanonicalStep,
    }),
    [
      adjustCanonicalStep,
      error,
      fontSizeStep,
      isHydrated,
      setCanonicalStep,
    ],
  );

  if (!isHydrated) {
    return (
      <div className="flex min-h-screen items-center justify-center text-sm text-muted-foreground">
        <span role="status">화면 설정을 불러오는 중입니다.</span>
      </div>
    );
  }

  return (
    <AppearancePreferencesContext.Provider value={value}>
      {children}
    </AppearancePreferencesContext.Provider>
  );
}

export function useAppearancePreferences() {
  return useContext(AppearancePreferencesContext);
}
