import { createSignal } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { api, EVENTS, type LogEntry } from "./api";

const MAX_ENTRIES = 2000;
/** A busy install logs in bursts: one update per burst instead of one per entry. */
const FLUSH_MS = 50;

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

/** Live entries waiting to be shown, applied together at most every FLUSH_MS. */
const pending: LogEntry[] = [];
let flushTimer: ReturnType<typeof setTimeout> | undefined;

function flush() {
  flushTimer = undefined;
  append(pending.splice(0));
}

function scheduleFlush() {
  flushTimer ??= setTimeout(flush, FLUSH_MS);
}

/** Subscribes to live logs, then backfills history. Seq numbers drop the overlap. */
export async function startLogStream() {
  if (started) return;
  started = true;
  let backfilled = false;
  await listen<LogEntry>(EVENTS.log, ({ payload }) => {
    pending.push(payload);
    if (backfilled) scheduleFlush();
  });
  append(await api.getLogs());
  backfilled = true;
  flush();
}

export const logEntries = entries;
