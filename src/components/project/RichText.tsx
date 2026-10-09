import { followLink } from "../../lib/projects";

/**
 * HTML written by a project's authors (description, changelog), already sanitized by the
 * backend. Links to Modrinth projects open in Tandem, the others in the browser.
 */
export default function RichText(props: { html: string; instanceId?: string; class?: string }) {
  return <div class={`md ${props.class ?? ""}`} innerHTML={props.html} onClick={(e) => followLink(e, props.instanceId)} />;
}
