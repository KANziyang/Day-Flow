import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errorText, native } from "../services/api";
import type { Snapshot } from "../types";
interface DataContextValue {
  data: Snapshot | null;
  error: string;
  loading: boolean;
  notice: string;
  refresh: () => Promise<void>;
  run: <T>(
    action: () => Promise<T>,
    success?: string,
  ) => Promise<T | undefined>;
  dismiss: () => void;
  reportError: (error: unknown) => void;
}
const Context = createContext<DataContextValue | null>(null);
export function DataProvider({ children }: { children: ReactNode }) {
  const [data, setData] = useState<Snapshot | null>(null),
    [error, setError] = useState("");
  const [loading, setLoading] = useState(true),
    [notice, setNotice] = useState("");
  const alive = useRef(true),
    generation = useRef(0);
  const reportError = useCallback((error: unknown) => { if (alive.current) setError(errorText(error)); }, []);
  const refresh = useCallback(async () => {
    const id = ++generation.current;
    try {
      const next = await api.snapshot();
      if (alive.current && id === generation.current) {
        setData(next);
        setError("");
      }
    } catch (e) {
      if (alive.current && id === generation.current) setError(errorText(e));
    } finally {
      if (alive.current) setLoading(false);
    }
  }, []);
  useEffect(() => {
    alive.current = true;
    const subscriptions: Promise<() => void>[] = [];
    if (native) {
      subscriptions.push(
        listen("data-changed", () => {
          void refresh();
        }),
      );
      subscriptions.push(
        listen<string>("app-error", (e) => setError(e.payload)),
      );
    }
    void Promise.all(subscriptions)
      .then(() => {
        if (alive.current) void refresh();
      })
      .catch((e) => {
        setError(errorText(e));
        setLoading(false);
      });
    const focus = () => {
      void refresh();
    };
    window.addEventListener("focus", focus);
    return () => {
      alive.current = false;
      window.removeEventListener("focus", focus);
      for (const subscription of subscriptions)
        void subscription.then((unlisten) => unlisten());
    };
  }, [refresh]);
  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(() => setNotice(""), 6000);
    return () => clearTimeout(timer);
  }, [notice]);
  useEffect(() => {
    const theme = data?.settings.theme ?? "system";
    const media = matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      document.documentElement.dataset.theme =
        theme === "system" ? (media.matches ? "dark" : "light") : theme;
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [data?.settings.theme]);
  const run: DataContextValue["run"] = useCallback(
    async (action, success) => {
      try {
        const result = await action();
        await refresh();
        if (alive.current && success) setNotice(success);
        return result;
      } catch (e) {
        if (alive.current) setError(errorText(e));
        return undefined;
      }
    },
    [refresh],
  );
  return (
    <Context
      value={{
        data,
        error,
        loading,
        notice,
        refresh,
        run,
        reportError,
        dismiss: () => setError(""),
      }}
    >
      {children}
    </Context>
  );
}
export function useData() {
  const value = useContext(Context);
  if (!value) throw new Error("缺少数据上下文");
  return value;
}
export function useClock() {
  const [now, setNow] = useState(new Date());
  useEffect(() => {
    const t = setInterval(() => setNow(new Date()), 30_000);
    const wake = () => setNow(new Date());
    window.addEventListener("focus", wake);
    return () => {
      clearInterval(t);
      window.removeEventListener("focus", wake);
    };
  }, []);
  return now;
}
