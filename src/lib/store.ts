import { createResource, createRoot, createSignal } from "solid-js";
import { api } from "./api";

export type Route =
  | { page: "home" }
  | { page: "instances" }
  | { page: "instance"; id: string }
  | { page: "discover"; instanceId?: string }
  | { page: "multi" }
  | { page: "settings" };

export const [route, navigate] = createSignal<Route>({ page: "home" });

/** App-wide data shared by the sidebar and the pages. */
export const { instances, refetchInstances, accounts, refetchAccounts } = createRoot(() => {
  const [instances, { refetch: refetchInstances }] = createResource(api.listInstances, {
    initialValue: [],
  });
  const [accounts, { refetch: refetchAccounts }] = createResource(api.listAccounts, {
    initialValue: [],
  });
  return { instances, refetchInstances, accounts, refetchAccounts };
});

export const activeAccount = () => accounts().find((a) => a.isActive);

/** Open state of the "new instance" dialog, with an optional preselected version. */
export const [newInstanceDialog, setNewInstanceDialog] = createSignal<{ version?: string } | null>(null);
