import Scene from "../components/Scene";

const COPY = {
  multi: {
    eyebrow: "JOUER À DEUX",
    title: "Ton monde, ton pote. Zéro serveur.",
    text: "Ouvre ton monde, envoie un code à ton ami : il te rejoint en direct, de PC à PC, sans serveur à louer.",
    scene: "night" as const,
  },
};

export default function Soon(props: { feature: keyof typeof COPY }) {
  const copy = () => COPY[props.feature];
  return (
    <section class="px-corners-lg relative h-[360px] bg-slate-700">
      <Scene variant={copy().scene} />
      <div class="absolute inset-0 bg-[linear-gradient(90deg,rgb(10_11_22/0.92)_0%,rgb(10_11_22/0.65)_45%,transparent_75%)]" />
      <div class="relative flex h-full max-w-[560px] flex-col justify-center gap-3 px-[34px]">
        <span class="flex items-center gap-2 font-pixel text-[13px] font-medium tracking-[2px] text-gold">
          <span class="size-1.5 bg-gold" />
          {copy().eyebrow} · BIENTÔT
        </span>
        <h1 class="font-pixel text-[40px] leading-tight font-bold [text-shadow:4px_4px_0_rgb(0_0_0/0.45)]">
          {copy().title}
        </h1>
        <p class="leading-relaxed text-[#C9CCD1]">{copy().text}</p>
      </div>
    </section>
  );
}
