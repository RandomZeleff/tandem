import { createResource, createSignal, For, Show } from "solid-js";
import { api, errorMessage } from "../lib/api";

export default function AccountMenu() {
  const [accounts, { refetch }] = createResource(api.listAccounts);
  const [open, setOpen] = createSignal(false);
  const [username, setUsername] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);

  const active = () => accounts()?.find((a) => a.isActive);

  async function run(action: () => Promise<unknown>) {
    setError(null);
    try {
      await action();
      await refetch();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function addOffline(e: SubmitEvent) {
    e.preventDefault();
    await run(async () => {
      await api.addOfflineAccount(username());
      setUsername("");
    });
  }

  return (
    <div class="relative">
      <button
        class="flex items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-neutral-800"
        onClick={() => setOpen((v) => !v)}
      >
        <span
          class="size-2 rounded-full"
          classList={{ "bg-emerald-400": !!active(), "bg-neutral-600": !active() }}
        />
        <span class={active() ? "text-neutral-200" : "text-neutral-400"}>
          {active()?.username ?? "Aucun compte"}
        </span>
      </button>

      <Show when={open()}>
        <div class="fixed inset-0 z-10" onClick={() => setOpen(false)} />
        <div class="absolute right-0 z-20 mt-1 w-72 rounded-lg border border-neutral-800 bg-neutral-900 p-3 shadow-xl">
          <Show when={(accounts()?.length ?? 0) > 0}>
            <ul class="mb-3 space-y-1">
              <For each={accounts()}>
                {(account) => (
                  <li class="group flex items-center gap-2 rounded px-2 py-1.5 hover:bg-neutral-800">
                    <button
                      class="flex flex-1 items-center gap-2 text-left text-sm"
                      onClick={() => run(() => api.setActiveAccount(account.id))}
                    >
                      <span
                        class="size-2 rounded-full"
                        classList={{
                          "bg-emerald-400": account.isActive,
                          "bg-neutral-700": !account.isActive,
                        }}
                      />
                      {account.username}
                      <span class="text-xs text-neutral-500">
                        {account.kind === "offline" ? "hors ligne" : "Microsoft"}
                      </span>
                    </button>
                    <button
                      class="text-xs text-neutral-500 opacity-0 group-hover:opacity-100 hover:text-red-400"
                      title="Retirer ce compte"
                      onClick={() => run(() => api.removeAccount(account.id))}
                    >
                      Retirer
                    </button>
                  </li>
                )}
              </For>
            </ul>
          </Show>

          <form onSubmit={addOffline} class="space-y-2">
            <label class="block text-xs text-neutral-400" for="offline-username">
              Ajouter un compte hors ligne
            </label>
            <div class="flex gap-2">
              <input
                id="offline-username"
                class="min-w-0 flex-1 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 text-sm outline-none focus:border-emerald-500"
                placeholder="Pseudo"
                maxLength={16}
                value={username()}
                onInput={(e) => setUsername(e.currentTarget.value)}
              />
              <button
                type="submit"
                class="rounded bg-neutral-800 px-3 py-1 text-sm hover:bg-neutral-700 disabled:opacity-40"
                disabled={username().trim().length < 3}
              >
                Ajouter
              </button>
            </div>
            <p class="text-xs text-neutral-500">
              La connexion Microsoft arrive bientôt.
            </p>
          </form>

          <Show when={error()}>
            <p class="mt-2 text-xs text-red-400">{error()}</p>
          </Show>
        </div>
      </Show>
    </div>
  );
}
