/** Fake project pages for the browser preview (see mock.ts), built from its catalogue. */
import type { DependencyItem, Instance, ProjectDetails, ProjectType, ProjectVersions, SearchHit } from "../lib/api";

const iso = (hoursAgo: number) => new Date(Date.now() - hoursAgo * 3_600_000).toISOString();
const slugify = (title: string) => title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");

const SKIES = ["#3E6FB0", "#E58B4B", "#1B2440", "#6FA8DC"];
function image(i: number): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 90" shape-rendering="crispEdges"><rect width="160" height="90" fill="${SKIES[i % 4]}"/><rect x="${30 + i * 25}" y="14" width="14" height="14" fill="#F2C744"/><rect y="56" width="160" height="34" fill="#5DBB3F"/><rect y="70" width="160" height="20" fill="#7A5420"/></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

/** What the backend sends: Markdown already rendered and sanitized. */
const BODY = [
  `<h2>Présentation</h2>`,
  `<p align="center"><img src="${image(0)}" alt="Bannière" width="480" loading="lazy"></p>`,
  `<p>Ce projet <strong>améliore les performances</strong> sans changer l'apparence du jeu. Il fonctionne avec <a href="https://modrinth.com/mod/iris-shaders">Iris</a> et la plupart des mods connus.</p>`,
  `<h3>Fonctions</h3>`,
  `<ul><li>Rendu des chunks bien plus rapide</li><li>Moins de saccades en exploration</li><li>Compatible avec <a href="https://modrinth.com/mod/mod-menu">Mod Menu</a></li></ul>`,
  `<a class="md-video" href="https://www.youtube.com/watch?v=dQw4w9WgXcQ"><img src="${image(2)}" alt="Vidéo YouTube"></a>`,
  `<h3>Comparaison</h3>`,
  `<table><thead><tr><th>Scène</th><th>Vanilla</th><th>Avec le mod</th></tr></thead><tbody><tr><td>Village</td><td>62 i/s</td><td>240 i/s</td></tr><tr><td>Jungle</td><td>41 i/s</td><td>180 i/s</td></tr></tbody></table>`,
  `<blockquote><p>Signalez les bugs sur <a href="https://github.com/example/issues">GitHub</a>.</p></blockquote>`,
  `<details><summary>Questions fréquentes</summary><p>Oui, il marche en solo comme en multijoueur.</p></details>`,
  `<pre><code>-XX:+UseG1GC -Xmx4G</code></pre>`,
].join("\n");

const CHANGELOG = `<ul><li>Corrige un plantage au chargement d'un monde</li><li>Ajoute la compatibilité avec <a href="https://modrinth.com/mod/iris-shaders">Iris</a> 1.8</li></ul><p>Merci à <strong>tous les testeurs</strong> !</p>`;

const GAMES = ["26.3", "26.2", "1.21.4", "1.21.1", "1.20.1"];

export function projectMocks(catalogue: Record<ProjectType, SearchHit[]>, instances: Instance[]) {
  /** A catalogue entry by id, slug or the start of its slug ("sodium", "iris"). */
  function find(id: string): { hit: SearchHit; type: ProjectType } | undefined {
    for (const [type, hits] of Object.entries(catalogue)) {
      const hit = hits.find((h) => h.projectId === id || slugify(h.title) === id || slugify(h.title).startsWith(`${id}-`));
      if (hit) return { hit, type: type as ProjectType };
    }
    return undefined;
  }

  function details(id: string): ProjectDetails {
    const found = find(id);
    if (!found) throw "project-not-found";
    const { hit, type } = found;
    return {
      id: hit.projectId,
      slug: slugify(hit.title),
      projectType: type,
      title: hit.title,
      summary: hit.description,
      // Dynamic FPS has no description, Lithium no gallery: empty states.
      bodyHtml: hit.projectId === "LQ3K71Q1" ? "" : BODY,
      iconUrl: hit.iconUrl,
      color: 0x5dbb3f,
      downloads: hit.downloads,
      followers: hit.follows,
      published: iso(24 * 900),
      updated: hit.dateModified,
      categories: hit.displayCategories.filter((c) => !["fabric", "iris", "optifine"].includes(c)),
      loaders:
        type === "mod" ? ["fabric", "quilt", "neoforge"] : type === "modpack" ? ["fabric"] : type === "shader" ? ["iris", "optifine"] : ["minecraft"],
      gameVersions: ["1.20.1", "1.21.1", "1.21.4", "26.2", "26.3"],
      clientSide: "required",
      serverSide: type === "mod" ? "unsupported" : "unknown",
      license: { id: "LGPL-3.0-only", name: "GNU Lesser General Public License v3.0 only", url: "https://spdx.org/licenses/LGPL-3.0-only.html" },
      links: [
        { kind: "source", label: "", url: "https://github.com/example/source" },
        { kind: "issues", label: "", url: "https://github.com/example/issues" },
        { kind: "discord", label: "", url: "https://discord.gg/example" },
        { kind: "donation", label: "Ko-fi", url: "https://ko-fi.com/example" },
      ],
      gallery:
        hit.projectId === "gvQqBUqZ"
          ? []
          : Array.from({ length: 5 }, (_, i) => ({
              thumbUrl: image(i),
              url: image(i),
              title: i === 3 ? null : `Capture ${i + 1}`,
              description: i === 0 ? "Une vue d'ensemble, avec les réglages par défaut." : null,
            })),
      organization: type === "mod" ? { name: "CaffeineMC", iconUrl: null, url: "https://modrinth.com/organization/caffeinemc" } : null,
      authors: [
        { name: hit.author, avatarUrl: null, role: "Owner", url: `https://modrinth.com/user/${hit.author}` },
        { name: "Contributeur", avatarUrl: null, role: "Developer", url: "https://modrinth.com/user/contrib" },
      ],
      url: `https://modrinth.com/${type}/${slugify(hit.title)}`,
    };
  }

  function versions(id: string, instanceId: string | null): ProjectVersions {
    const project = details(id);
    const instance = instances.find((i) => i.id === instanceId);
    const loaders = project.projectType === "mod" ? ["fabric", "neoforge"] : project.loaders;
    const list = Array.from({ length: 40 }, (_, i) => {
      const game = GAMES[Math.floor(i / 8)];
      const loader = loaders[i % loaders.length];
      const number = `${6 - Math.floor(i / 8)}.${7 - (i % 8)}.0`;
      const fitsLoader = project.projectType !== "mod" || loader === instance?.loader || (loader === "fabric" && instance?.loader === "quilt");
      return {
        id: `${project.id}-v${i}`,
        name: `${project.title} ${number} pour ${game}`,
        versionNumber: `${number}+${game}`,
        versionType: i % 7 === 0 ? "beta" : i % 11 === 3 ? "alpha" : "release",
        gameVersions: i % 9 === 0 ? [game, "1.21.3"] : [game],
        loaders: [loader],
        datePublished: iso(i * 60 + 5),
        downloads: Math.round(400_000 / (i + 1)),
        fileName: `${project.slug}-${number}.jar`,
        size: 1_200_000 + i * 3000,
        hasChangelog: i % 5 !== 4,
        compatible: project.projectType === "modpack" || !instance || (instance.gameVersion === game && fitsLoader),
      };
    });
    const recommended = list.find((v) => v.compatible && v.versionType === "release") ?? list.find((v) => v.compatible);
    return { versions: list, recommended: recommended?.id ?? null };
  }

  const summary = (hit: SearchHit, projectType: string) => ({
    id: hit.projectId,
    slug: slugify(hit.title),
    title: hit.title,
    summary: hit.description,
    projectType,
    iconUrl: hit.iconUrl,
    downloads: hit.downloads,
  });

  function dependencies(projectId: string): DependencyItem[] {
    const project = details(projectId);
    if (project.projectType === "modpack") {
      const mods = Array.from({ length: 120 }, (_, i): DependencyItem => {
        const h = catalogue.mod[i % catalogue.mod.length];
        const title = i < catalogue.mod.length ? h.title : `${h.title} Addon ${i}`;
        return { kind: "embedded", project: { ...summary(h, "mod"), id: `${h.projectId}-${i}`, title } };
      });
      return [
        ...mods,
        { kind: "embedded", project: summary(catalogue.resourcepack[0], "resourcepack") },
        { kind: "embedded", project: summary(catalogue.shader[0], "shader") },
      ];
    }
    if (project.projectType !== "mod" || project.id === "P7dR8mSH") return [];
    return [
      { kind: "required", project: summary(catalogue.mod[0], "mod") },
      { kind: "optional", project: summary(catalogue.mod[3], "mod") },
      { kind: "incompatible", project: summary(catalogue.mod[6], "mod") },
    ];
  }

  const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

  /** Answers the project commands; undefined for any other command. */
  return async (cmd: string, args: Record<string, unknown>): Promise<unknown> => {
    switch (cmd) {
      case "project_details":
        await wait(350);
        return details(args.id as string);
      case "project_versions":
        await wait(450);
        return versions(args.projectId as string, args.instanceId as string | null);
      case "version_changelog":
        await wait(250);
        return CHANGELOG;
      case "version_dependencies":
        await wait(400);
        return dependencies(args.projectId as string);
      case "plugin:opener|open_url":
        // CurseForge file pages stand for a download by hand (see the manual downloads mock).
        if ((args.url as string).includes("curseforge.com")) window.dispatchEvent(new Event("mock:manual-download"));
        else window.open(args.url as string, "_blank", "noopener");
        return null;
      default:
        return undefined;
    }
  };
}
