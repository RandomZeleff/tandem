import { listen } from "@tauri-apps/api/event";
import { createSignal } from "solid-js";
import { api, errorMessage, type FriendsOverview } from "./api";
import { duo, joinFriend } from "./duo";
import { notifyInGame } from "./notify";
import { navigate } from "./store";
import { toast } from "./toast";

/** An invite received from a friend, kept until it is used or the friend hosts no more. */
export interface FriendInvite {
  id: string;
  name: string;
  code: string;
  instance: string | null;
  world: string | null;
}

type FriendEvent =
  | { type: "changed" }
  | { type: "request"; id: string; name: string }
  | ({ type: "invite" } & FriendInvite);

const EMPTY: FriendsOverview = { available: false, code: null, friends: [], hidden: false, openToFriends: false };

const [friends, setFriends] = createSignal<FriendsOverview>(EMPTY);
const [invites, setInvites] = createSignal<FriendInvite[]>([]);
export { friends, invites };

/** Friends online, for badges. */
export const friendsOnline = () => friends().friends.filter((f) => f.state === "friend" && f.presence).length;
/** Requests and invites waiting for an answer. */
export const friendsPending = () =>
  friends().friends.filter((f) => f.state === "incoming").length + invites().length;

export async function refreshFriends() {
  try {
    setFriends(await api.friendsOverview());
  } catch {
    // Kept as it was: the next event or visit tries again.
  }
}

/** Joins the world a friend invited the player to (or opened to friends). */
export async function joinInvite(invite: Pick<FriendInvite, "code" | "id">) {
  setInvites((list) => list.filter((i) => i.id !== invite.id));
  navigate({ page: "multi" });
  try {
    await joinFriend(invite.code);
  } catch (err) {
    toast(errorMessage(err), { tone: "error" });
  }
}

export const dismissInvite = (id: string) => setInvites((list) => list.filter((i) => i.id !== id));

let started = false;

/** Listens to the friends service and loads the list, once. */
export async function startFriendsEvents() {
  if (started) return;
  started = true;
  await listen<FriendEvent>("friends://event", ({ payload }) => {
    switch (payload.type) {
      case "changed":
        void refreshFriends();
        break;
      case "request":
        toast(`${payload.name} veut devenir ton ami`, {
          tone: "info",
          action: { label: "Voir", run: () => navigate({ page: "multi" }) },
        });
        void notifyInGame(`${payload.name} veut devenir ton ami`, "Réponds-lui dans Jouer ensemble.");
        void refreshFriends();
        break;
      case "invite": {
        if (duo.guest) break;
        const invite: FriendInvite = payload;
        setInvites((list) => [...list.filter((i) => i.id !== invite.id), invite]);
        const where = invite.instance ? ` sur ${invite.instance}` : "";
        toast(`${invite.name} t'invite${where}`, {
          tone: "info",
          durationMs: 15000,
          action: { label: "Rejoindre", run: () => void joinInvite(invite) },
        });
        void notifyInGame(`${invite.name} t'invite${where}`, "Ouvre Tandem pour rejoindre sa partie.");
        break;
      }
    }
  });
  await refreshFriends();
}
