import { listen } from "@tauri-apps/api/event";
import { createSignal } from "solid-js";
import { api, errorMessage, type Account } from "./api";
import { refetchAccounts } from "./store";

/** Where a Microsoft sign-in is. */
export type LoginStep = "browser" | "microsoft" | "xbox" | "minecraft" | "profile";

export type LoginState =
  | { kind: "idle" }
  /** Waiting for the player in the browser; `url` is empty until the page is ready. */
  | { kind: "browser"; url: string }
  | { kind: "working"; step: Exclude<LoginStep, "browser"> }
  | { kind: "done"; account: Account }
  | { kind: "error"; message: string };

const [login, setLogin] = createSignal<LoginState>({ kind: "idle" });
export { login };

/** Why the accounts window was opened, when it was not the player's own click. */
export type AccountsReason = "needed" | "expired";

/** The accounts window: `null` when closed. */
export const [accountsWindow, setAccountsWindow] = createSignal<{ reason?: AccountsReason } | null>(null);

export const openAccounts = (reason?: AccountsReason) => setAccountsWindow(reason ? { reason } : {});

let cancelled = false;
let listening = false;

async function listenSteps() {
  if (listening) return;
  listening = true;
  await listen<LoginStep>("auth://step", ({ payload }) => {
    // Only while signing in: a late event must not cover the result or the error.
    const kind = login().kind;
    if (payload !== "browser" && (kind === "browser" || kind === "working")) setLogin({ kind: "working", step: payload });
  });
}

/** Signs in with Microsoft in the browser. The account becomes the active one. */
export async function signInWithMicrosoft() {
  cancelled = false;
  await listenSteps();
  setLogin({ kind: "browser", url: "" });
  try {
    const url = await api.beginMicrosoftLogin();
    if (cancelled) return;
    setLogin({ kind: "browser", url });
    const account = await api.finishMicrosoftLogin();
    await refetchAccounts();
    setLogin({ kind: "done", account });
  } catch (err) {
    setLogin(cancelled ? { kind: "idle" } : { kind: "error", message: errorMessage(err) });
  }
}

export function cancelSignIn() {
  cancelled = true;
  void api.cancelMicrosoftLogin().catch(() => {});
  setLogin({ kind: "idle" });
}

/** Back to the account list after a finished or failed sign-in. */
export const resetSignIn = () => setLogin({ kind: "idle" });

/** Splits a message on its web links, so they can be made clickable. */
export function withLinks(message: string): { text: string; url?: string }[] {
  const parts: { text: string; url?: string }[] = [];
  let last = 0;
  for (const match of message.matchAll(/https:\/\/[^\s)]+[^\s).,]/g)) {
    if (match.index > last) parts.push({ text: message.slice(last, match.index) });
    parts.push({ text: match[0].replace(/^https:\/\/(www\.)?/, ""), url: match[0] });
    last = match.index + match[0].length;
  }
  if (last < message.length) parts.push({ text: message.slice(last) });
  return parts;
}
