import { Show } from "solid-js";
import type { Account } from "../../lib/api";
import { skinLook } from "../../lib/look";
import { SkinHead } from "../pixel";

/** The Microsoft logo, as Microsoft asks it to appear on sign-in buttons. */
export function MicrosoftLogo(props: { size?: number; class?: string }) {
  return (
    <svg width={props.size ?? 16} height={props.size ?? 16} viewBox="0 0 21 21" shape-rendering="crispEdges" aria-hidden="true" class={props.class}>
      <rect x="0" y="0" width="10" height="10" fill="#F25022" />
      <rect x="11" y="0" width="10" height="10" fill="#7FBA00" />
      <rect x="0" y="11" width="10" height="10" fill="#00A4EF" />
      <rect x="11" y="11" width="10" height="10" fill="#FFB900" />
    </svg>
  );
}

/** Tandem's mark (the two blocks of the title bar). */
export function TandemMark(props: { size?: number }) {
  const width = () => props.size ?? 36;
  return (
    <svg width={width()} height={(width() * 2) / 3} viewBox="0 0 12 8" shape-rendering="crispEdges" aria-hidden="true">
      <rect x="0" y="2" width="6" height="6" fill="#2E6B1E" />
      <rect x="0" y="2" width="6" height="2" fill="#5DBB3F" />
      <rect x="1" y="2" width="1" height="1" fill="var(--color-xp)" />
      <rect x="6" y="0" width="6" height="6" fill="#7A5420" />
      <rect x="6" y="0" width="6" height="2" fill="#F2C744" />
      <rect x="10" y="0" width="1" height="1" fill="#FFE38A" />
      <rect x="5" y="2" width="1" height="4" fill="#0B0C0E" />
    </svg>
  );
}

/** A grass block seen from the front, for the Minecraft step of the sign-in. */
export function GrassBlock(props: { size?: number }) {
  return (
    <svg width={props.size ?? 16} height={props.size ?? 16} viewBox="0 0 8 8" shape-rendering="crispEdges" aria-hidden="true">
      <rect width="8" height="8" fill="#7A5420" />
      <rect width="8" height="3" fill="#5DBB3F" />
      <path d="M0 3h1v1H0zM2 3h1v2H2zM5 3h1v1H5zM7 3h1v2H7z" fill="#5DBB3F" />
      <path d="M1 0h1v1H1zM4 1h1v1H4zM6 0h1v1H6z" fill="#8BE04E" />
      <path d="M1 5h1v1H1zM4 6h1v1H4zM6 5h1v1H6z" fill="#5C3D16" />
    </svg>
  );
}

/** The real face of a Microsoft account's skin, or a head drawn from the name. */
export function AccountAvatar(props: { account: Pick<Account, "username" | "avatar">; size: number }) {
  return (
    <Show when={props.account.avatar} fallback={<SkinHead look={skinLook(props.account.username)} size={props.size} />}>
      {(src) => (
        <img
          src={src()}
          width={props.size}
          height={props.size}
          alt=""
          draggable={false}
          style={{ "image-rendering": "pixelated" }}
        />
      )}
    </Show>
  );
}

/** "Microsoft" with its logo, or "Hors ligne". */
export function AccountKindLabel(props: { account: Pick<Account, "kind"> }) {
  return (
    <span class="inline-flex items-center gap-1.5">
      <Show when={props.account.kind === "microsoft"} fallback={<span class="size-2 bg-faint" aria-hidden="true" />}>
        <MicrosoftLogo size={10} />
      </Show>
      {props.account.kind === "microsoft" ? "Compte Microsoft" : "Hors ligne"}
    </span>
  );
}

/** White button in Microsoft's own style, as their sign-in guidelines ask. */
export function MicrosoftButton(props: { label?: string; disabled?: boolean; onClick: () => void; class?: string }) {
  return (
    <button class={`btn btn-microsoft px-corners ${props.class ?? ""}`} disabled={props.disabled} onClick={props.onClick}>
      <MicrosoftLogo size={16} />
      {props.label ?? "Se connecter avec Microsoft"}
    </button>
  );
}
