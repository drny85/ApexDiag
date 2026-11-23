import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { api, asCmdError, type Catalog, type ConnectReport, type MarkInfo, type Replay, type StateReport } from "./tauri";

/** PIDs streamed while connected, in priority order. */
export const LIVE_PIDS = [0x0c, 0x0d, 0x05, 0x42, 0x06, 0x07, 0x08, 0x09, 0x0b, 0x10, 0x0e, 0x04, 0x14, 0x5c, 0x0f, 0x2f];

const HISTORY_SECONDS = 30;

export interface Sample {
  t: number;
  v: number;
}

interface DiagContextValue {
  catalog: Catalog | null;
  connection: ConnectReport | null;
  setConnection: (c: ConnectReport | null) => void;
  /** Latest value per PID. */
  live: Map<number, number>;
  /** Rolling history per PID (mutable ref, read inside animation frames). */
  history: React.RefObject<Map<number, Sample[]>>;
  vehicle: StateReport | null;
  refreshVehicle: () => Promise<void>;
  /** Bumped whenever something may have changed the car (clear, run). */
  revision: number;
  bump: () => void;
  /** A fault picked elsewhere to open in the Repair Copilot. */
  focus: { code: string; module: string | null } | null;
  setFocus: (f: { code: string; module: string | null } | null) => void;
  /** Instant Replay: the moment being recorded, if any. */
  recording: (MarkInfo & { started: number }) | null;
  /** Instant Replay: saving (reading fault memory) after the window ends. */
  savingReplay: boolean;
  /** The replay saved most recently in this session. */
  lastReplay: Replay | null;
  replayError: string | null;
  markMoment: () => Promise<void>;
  /** Stop recording now and save what is there. */
  finishReplay: () => Promise<void>;
  openReplay: string | null;
  setOpenReplay: (id: string | null) => void;
}

const DiagContext = createContext<DiagContextValue | null>(null);

export function DiagProvider({ children }: { children: ReactNode }) {
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [connection, setConnection] = useState<ConnectReport | null>(null);
  const [live, setLive] = useState<Map<number, number>>(new Map());
  const [vehicle, setVehicle] = useState<StateReport | null>(null);
  const [revision, setRevision] = useState(0);
  const [focus, setFocus] = useState<{ code: string; module: string | null } | null>(null);
  const history = useRef<Map<number, Sample[]>>(new Map());
  const [recording, setRecording] = useState<(MarkInfo & { started: number }) | null>(null);
  const [savingReplay, setSavingReplay] = useState(false);
  const [lastReplay, setLastReplay] = useState<Replay | null>(null);
  const [replayError, setReplayError] = useState<string | null>(null);
  const [openReplay, setOpenReplay] = useState<string | null>(null);
  const finishing = useRef(false);

  const finishReplay = useCallback(async () => {
    if (finishing.current) return;
    finishing.current = true;
    setSavingReplay(true);
    try {
      const r = await api.replayFinish(true);
      setLastReplay(r);
      setReplayError(null);
    } catch (e) {
      setReplayError(asCmdError(e).message);
    } finally {
      setRecording(null);
      setSavingReplay(false);
      finishing.current = false;
    }
  }, []);

  const markMoment = useCallback(async () => {
    if (recording || finishing.current) return;
    try {
      const m = await api.replayMark();
      setReplayError(null);
      setRecording({ ...m, started: Date.now() });
    } catch (e) {
      setReplayError(asCmdError(e).message);
    }
  }, [recording]);

  // Save automatically once the 30 seconds after the mark are recorded.
  useEffect(() => {
    if (!recording) return;
    const left = recording.after_ms - (Date.now() - recording.started);
    const id = setTimeout(finishReplay, Math.max(0, left) + 300);
    return () => clearTimeout(id);
  }, [recording, finishReplay]);

  useEffect(() => {
    api.catalog().then(setCatalog).catch(() => {});
  }, []);

  // Stream live data while connected.
  useEffect(() => {
    if (!connection) {
      setLive(new Map());
      history.current = new Map();
      setRecording(null);
      return;
    }
    const supported = new Set(connection.supported_pids);
    const pids = LIVE_PIDS.filter((p) => supported.size === 0 || supported.has(p));
    let stop = false;
    (async () => {
      while (!stop) {
        const started = performance.now();
        try {
          const values = await api.readLive(pids);
          const now = performance.now();
          const next = new Map<number, number>();
          for (const { pid, value } of values) {
            next.set(pid, value);
            const h = history.current.get(pid) ?? [];
            h.push({ t: now, v: value });
            while (h.length && h[0]!.t < now - HISTORY_SECONDS * 1000) h.shift();
            history.current.set(pid, h);
          }
          if (!stop) setLive(next);
        } catch {
          /* adapter hiccup: try again next tick */
        }
        const wait = Math.max(0, 120 - (performance.now() - started));
        await new Promise((r) => setTimeout(r, wait));
      }
    })();
    return () => {
      stop = true;
    };
  }, [connection]);

  const refreshVehicle = useCallback(async () => {
    try {
      setVehicle(await api.vehicleState());
    } catch {
      setVehicle(null);
    }
  }, []);

  // Safety-gate state, refreshed in the background.
  useEffect(() => {
    if (!connection) {
      setVehicle(null);
      return;
    }
    refreshVehicle();
    const id = setInterval(refreshVehicle, 1500);
    return () => clearInterval(id);
  }, [connection, refreshVehicle, revision]);

  const value = useMemo(
    () => ({
      catalog,
      connection,
      setConnection,
      live,
      history,
      vehicle,
      refreshVehicle,
      revision,
      bump: () => setRevision((r) => r + 1),
      focus,
      setFocus,
      recording,
      savingReplay,
      lastReplay,
      replayError,
      markMoment,
      finishReplay,
      openReplay,
      setOpenReplay,
    }),
    [catalog, connection, live, vehicle, refreshVehicle, revision, focus, recording, savingReplay, lastReplay, replayError, markMoment, finishReplay, openReplay],
  );

  return <DiagContext.Provider value={value}>{children}</DiagContext.Provider>;
}

export function useDiag() {
  const ctx = useContext(DiagContext);
  if (!ctx) throw new Error("useDiag outside DiagProvider");
  return ctx;
}

/** The vehicle pack of the connected car. */
export function usePack() {
  const { connection } = useDiag();
  return connection?.vehicle.pack ?? null;
}

export function usePid(pid: number) {
  const { catalog } = useDiag();
  return catalog?.pids.find((p) => p.pid === pid);
}
