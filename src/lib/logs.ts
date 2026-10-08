import { createSignal } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { api, EVENTS, type LogEntry } from "./api";

const MAX_ENTRIES = 2000;

const [entries, setEntries] = createSignal<LogEntry[]>([]);
let lastSeq = 0;
let started = false;

function append(batch: LogEntry[]) {
  const fresh = batch.filter((e) => e.seq > lastSeq);
  if (fresh.length === 0) return;
  lastSeq = fresh[fresh.length - 1].seq;
  setEntries((prev) => {
    const next = prev.concat(fresh);
    return next.length > MAX_ENTRIES ? next.slice(next.length - MAX_ENTRIES) : next;
  });
}

/** Subscribes to live logs, then backfills history. Seq numbers drop the overlap. */
export async function startLogStream() {
  if (started) return;
  started = true;
  const pending: LogEntry[] = [];
  let backfilled = false;
  await listen<LogEntry>(EVENTS.log, ({ payload }) => {
    if (backfilled) append([payload]);
    else pending.push(payload);
  });
  append(await api.getLogs());
  backfilled = true;
  append(pending);
}

export const logEntries = entries;
