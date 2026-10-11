package tandem.optimizations;

import java.lang.invoke.MethodHandle;
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.lang.ref.WeakReference;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

import net.minecraft.class_1299;
import net.minecraft.class_1311;
import net.minecraft.class_1959;
import net.minecraft.class_2378;
import net.minecraft.class_3108;
import net.minecraft.class_3195;
import net.minecraft.class_3218;
import net.minecraft.class_6012;
import net.minecraft.class_7061;
import net.minecraft.class_7924;
import net.minecraft.server.MinecraftServer;

/**
 * Spawn groups whose natural spawns can never succeed: every entity type the biomes and
 * structures list for them is refused by a mob-blocking mod (MobsBeGone), or the lists
 * are empty. Vanilla still runs the whole spawn search for them on every tick around
 * every player, then the entity is thrown away. Skipping them changes nothing in game.
 */
public final class DeadSpawnGroups {
    private static WeakReference<MinecraftServer> server = new WeakReference<>(null);
    private static Set<class_1311> dead = EnumSet.noneOf(class_1311.class);
    private static final MethodHandle BLOCKED = findBlocker();

    private DeadSpawnGroups() {}

    public static boolean isDead(class_1311 group, class_3218 world) {
        try {
            MinecraftServer s = world.method_8503();
            if (server.get() != s) {
                dead = compute(s);
                server = new WeakReference<>(s);
            }
        } catch (LinkageError e) {
            // A Minecraft version where a method used here is missing: skip nothing.
            return false;
        }
        return dead.contains(group);
    }

    private static Set<class_1311> compute(MinecraftServer s) {
        Set<class_1311> out = EnumSet.noneOf(class_1311.class);
        try {
            class_2378<class_1959> biomes = s.method_30611().method_30530(class_7924.field_41236);
            class_2378<class_3195> structures = s.method_30611().method_30530(class_7924.field_41246);
            for (class_1311 group : class_1311.values()) {
                List<class_1299<?>> types = new ArrayList<>();
                biomes.method_10220().forEach(b -> collect(b.method_30966().method_31004(group), types));
                structures.method_10220().forEach(st -> {
                    Map<class_1311, class_7061> spawns = st.method_41615();
                    class_7061 sp = spawns.get(group);
                    if (sp != null) collect(sp.comp_515(), types);
                });
                // Nether brick spawns, hard-coded in SpawnHelper.getSpawnEntries.
                if (group == class_1311.field_6302) collect(class_3108.field_13705, types);
                if (types.stream().allMatch(DeadSpawnGroups::blocked)) out.add(group);
            }
        } catch (Throwable t) {
            TandemOptimizations.LOG.warn("spawn group check failed, nothing skipped", t);
            return EnumSet.noneOf(class_1311.class);
        }
        TandemOptimizations.LOG.info("natural spawning skipped for {} (nothing in them can spawn)", out);
        return out;
    }

    /** Adds the entity type of every entry, or null (never dead) if it cannot be read. */
    private static void collect(class_6012<?> pool, List<class_1299<?>> into) {
        for (Object entry : pool.method_34994()) into.add(entityType(entry, 0));
    }

    /**
     * The entity type inside a spawn entry, read by reflection: entries changed shape in
     * 1.21.5 (a spawn entry inside a weighted wrapper, both records).
     */
    private static class_1299<?> entityType(Object o, int depth) {
        if (o instanceof class_1299<?> type) return type;
        if (o == null || depth > 2 || o instanceof Number || o instanceof Boolean) return null;
        for (Class<?> c = o.getClass(); c != null && c != Object.class; c = c.getSuperclass()) {
            for (Field f : c.getDeclaredFields()) {
                if (Modifier.isStatic(f.getModifiers()) || f.getType().isPrimitive()) continue;
                try {
                    f.setAccessible(true);
                    class_1299<?> type = entityType(f.get(o), depth + 1);
                    if (type != null) return type;
                } catch (ReflectiveOperationException | RuntimeException e) {
                    return null;
                }
            }
        }
        return null;
    }

    private static boolean blocked(class_1299<?> type) {
        if (BLOCKED == null || type == null) return false;
        try {
            return (boolean) BLOCKED.invoke(type);
        } catch (Throwable t) {
            return false;
        }
    }

    private static MethodHandle findBlocker() {
        try {
            Class<?> c = Class.forName("moe.okaeri.mobsbegone.MobsBeGone");
            return MethodHandles.publicLookup().findStatic(c, "isEntityBlacklisted",
                    MethodType.methodType(boolean.class, class_1299.class));
        } catch (ReflectiveOperationException | LinkageError e) {
            return null;
        }
    }
}
