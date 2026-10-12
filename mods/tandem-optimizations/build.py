"""Builds the Fabric jar of Tandem Optimizations, without Gradle.

The sources use Fabric's intermediary names (`class_1308`, `method_5773`...), which is
what Fabric runs in production from 1.14 to 1.21.11: javac compiles them as they are,
no remapping. The jar is reproducible (fixed timestamps), so rebuilding unchanged
sources gives the same file.

    python build.py --mc <intermediary.jar> [--check <dir>] --libs <libraries dir>
                    [--jdk <JDK 17+ dir>]

--mc: a Minecraft jar in intermediary names, e.g. the one Fabric writes on first launch
  in `.fabric/remappedJars/minecraft-<version>-<loader>/` (client or server jar).
--check: a folder searched for more `*-intermediary.jar` (one per Minecraft version).
  The sources are compiled against each of them and the Mixin targets looked up in each
  (see TARGETS): a method missing in a version fails the build instead of turning the
  optimization off in game. Run it against every version from 1.20 to 1.21.11 before
  committing a new jar.
--libs: a libraries folder with Fabric Loader, Mixin and slf4j (Tandem's `libraries`).

Output: dist/tandem-optimizations-fabric.jar (embedded in Tandem, see crates/tandem-core).
"""

import argparse
import io
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE / "dist" / "tandem-optimizations-fabric.jar"
STAMP = (2026, 1, 1, 0, 0, 0)

# Mixin targets are strings javac cannot check: (class, method, text its body or
# signature must contain). Keep in sync with the mixins.
TARGETS = [
    ("net.minecraft.server.MinecraftServer", "method_3748(", "(Ljava/util/function/BooleanSupplier;)V"),
    ("net.minecraft.class_3218", "method_18762(", "net/minecraft/class_1297.method_5773:()V"),
    ("net.minecraft.class_1948", "method_8663(",
     "(Lnet/minecraft/class_1311;Lnet/minecraft/class_3218;Lnet/minecraft/class_2818;"
     "Lnet/minecraft/class_1948$class_5261;Lnet/minecraft/class_1948$class_5259;)V"),
]


def newest(libs: Path, pattern: str) -> Path:
    def key(p: Path):
        return [(0, int(x), "") if x.isdigit() else (1, 0, x) for x in p.parent.name.replace("+", ".").split(".")]

    found = sorted(libs.rglob(pattern), key=key)
    if not found:
        sys.exit(f"{pattern} not found under {libs}")
    return found[-1]


def classpath(libs: Path, tmp: Path) -> list[Path]:
    loader = newest(libs, "fabric-loader-*.jar")
    # Minecraft's own dependencies that show up in the signatures the sources touch.
    names = ["sponge-mixin", "slf4j-api", "datafixerupper", "fastutil", "guava", "gson", "brigadier",
             "authlib", "logging", "joml", "asm", "asm-tree"]
    jars = [loader, *(newest(libs, f"{n}-[0-9]*.jar") for n in names)]
    with zipfile.ZipFile(loader) as z:
        nested = [n for n in z.namelist() if "mixinextras" in n and n.endswith(".jar")]
        if not nested:
            sys.exit("MixinExtras not found in Fabric Loader (0.15+ needed)")
        extras = tmp / "mixinextras.jar"
        extras.write_bytes(z.read(nested[0]))
        jars.append(extras)
    return jars


def javac(jdk: str | None) -> str:
    if jdk:
        return str(Path(jdk) / "bin" / ("javac.exe" if os.name == "nt" else "javac"))
    found = shutil.which("javac")
    if not found:
        sys.exit("javac not found: pass --jdk")
    return found


def compile_against(javac_bin: str, mc: Path, cp: list[Path], out: Path) -> str | None:
    sources = sorted(str(p) for p in (HERE / "src/main/java").rglob("*.java"))
    out.mkdir(parents=True, exist_ok=True)
    cmd = [javac_bin, "--release", "17", "-nowarn", "-proc:none", "-encoding", "UTF-8",
           "-cp", os.pathsep.join(str(p) for p in [mc, *cp]), "-d", str(out), *sources]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        return f"compile failed against {mc}:\n{result.stdout}{result.stderr}"
    return None


def check_targets(javap_bin: str, mc: Path) -> str | None:
    for cls, method, needle in TARGETS:
        out = subprocess.run([javap_bin, "-p", "-c", "-s", "-cp", str(mc), cls],
                             capture_output=True, text=True).stdout
        bodies = [b for b in out.split("\n\n") if method in b.strip().split("\n", 1)[0]]
        if not any(needle in b for b in bodies):
            return f"Mixin target missing in {mc}: {cls}.{method} with {needle}"
    return None


def references(javap_bin: str, classes: Path) -> set[tuple[str, str, str]]:
    """Minecraft members the built classes use: (owner, name, descriptor)."""
    files = [str(p) for p in classes.rglob("*.class")]
    out = subprocess.run([javap_bin, "-c", "-p", *files], capture_output=True, text=True).stdout
    refs = set()
    for m in re.finditer(r"// (?:Method|InterfaceMethod|Field) (net/minecraft/[\w$/]+)\.(\w+):(\S+)", out):
        refs.add((m.group(1).replace("/", "."), m.group(2), m.group(3)))
    return refs


def check_references(javap_bin: str, mc: Path, refs: set[tuple[str, str, str]]) -> str | None:
    """Every member the built jar uses exists in this version (in its class or a parent)."""
    cache: dict[str, tuple[list[str], set[tuple[str, str]]]] = {}

    def members(cls: str):
        if cls not in cache:
            out = subprocess.run([javap_bin, "-p", "-s", "-cp", str(mc), cls], capture_output=True, text=True).stdout
            header = next((l for l in out.splitlines() if l.rstrip().endswith("{")), "")
            parents = re.findall(r"(net\.minecraft\.[\w$.]+)", header.split(cls, 1)[-1])
            found, name = set(), None
            for line in out.splitlines():
                if line.strip().startswith("descriptor:") and name:
                    found.add((name, line.split(":", 1)[1].strip()))
                elif "(" in line or line.rstrip().endswith(";"):
                    m = re.search(r"(\w+)(?:\(|;)", line.split("=")[0])
                    name = m.group(1) if m else None
            cache[cls] = (parents, found)
        return cache[cls]

    def has(cls: str, name: str, desc: str) -> bool:
        parents, found = members(cls)
        return (name, desc) in found or any(has(p.split("<")[0], name, desc) for p in parents)

    missing = [f"{o}.{n}{d}" for o, n, d in sorted(refs) if not has(o, n, d)]
    return f"missing in {mc}: {', '.join(missing)}" if missing else None


def write_jar(classes: Path):
    files = sorted([p for p in classes.rglob("*") if p.is_file()] +
                   [p for p in (HERE / "src/main/resources").rglob("*") if p.is_file()],
                   key=lambda p: p.name)
    entries = {}
    for p in files:
        root = classes if classes in p.parents else HERE / "src/main/resources"
        entries[p.relative_to(root).as_posix()] = p.read_bytes()
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as z:
        manifest = zipfile.ZipInfo("META-INF/MANIFEST.MF", STAMP)
        z.writestr(manifest, "Manifest-Version: 1.0\r\n\r\n", zipfile.ZIP_DEFLATED)
        for name in sorted(entries):
            z.writestr(zipfile.ZipInfo(name, STAMP), entries[name], zipfile.ZIP_DEFLATED)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(buf.getvalue())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--mc", required=True, type=Path)
    ap.add_argument("--check", type=Path)
    ap.add_argument("--libs", required=True, type=Path)
    ap.add_argument("--jdk")
    args = ap.parse_args()
    javac_bin = javac(args.jdk)
    javap_bin = str(Path(javac_bin).with_name(Path(javac_bin).name.replace("javac", "javap")))
    jars = [args.mc, *(sorted(args.check.rglob("*-intermediary.jar")) if args.check else [])]
    with tempfile.TemporaryDirectory() as t:
        tmp = Path(t)
        cp = classpath(args.libs, tmp)
        error = compile_against(javac_bin, args.mc, cp, tmp / "classes0")
        if error:
            sys.exit(error)
        refs = references(javap_bin, tmp / "classes0")
        failed = False
        for i, mc in enumerate(jars):
            error = (compile_against(javac_bin, mc, cp, tmp / f"classes{i}") or check_targets(javap_bin, mc)
                     or check_references(javap_bin, mc, refs))
            failed |= error is not None
            print(error or f"ok: {mc.parent.name}")
        if failed:
            sys.exit("not built: fix the versions above")
        write_jar(tmp / "classes0")
    print(f"{OUT} ({OUT.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
