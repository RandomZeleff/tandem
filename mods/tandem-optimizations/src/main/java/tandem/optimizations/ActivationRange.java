package tandem.optimizations;

import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

import net.minecraft.class_1296;
import net.minecraft.class_1297;
import net.minecraft.class_1308;
import net.minecraft.class_1321;
import net.minecraft.class_1428;
import net.minecraft.class_1429;
import net.minecraft.class_1510;
import net.minecraft.class_1528;
import net.minecraft.class_1550;
import net.minecraft.class_1641;
import net.minecraft.class_1657;
import net.minecraft.class_3218;
import net.minecraft.class_3763;
import net.minecraft.class_3988;
import net.minecraft.class_4760;
import net.minecraft.class_5418;
import net.minecraft.class_7260;

/**
 * Entity activation range, the idea of Spigot and Paper: a mob far from every player,
 * standing idle, ticks once every {@link #INTERVAL} ticks instead of every tick. That is
 * most of the mobs a server simulates (they spawn 24 to 128 blocks from players).
 *
 * <p>Kept close to vanilla:
 * <ul>
 *   <li>Only vanilla mobs: items, projectiles, minecarts and every modded entity always
 *       tick. A mod may count time in its own entity's tick (Cobblemon ages its Pokemon
 *       that way: slowed down, they despawned four times later and piled up), and there
 *       is no way to know which.
 *   <li>Distance is horizontal: a mob far below or above a player (mob farm towers)
 *       stays active.
 *   <li>A mob ticks when its own age is a multiple of {@link #INTERVAL}, so the game's
 *       periodic actions ({@code age % 20 == 0}...) still happen on time.
 *   <li>Never slowed: in the air, in water or lava, on fire, just hurt, with a target, a
 *       status effect, a rider or a vehicle, tamed, in love, villagers and wandering
 *       traders (trades, farms), raiders (raids), bosses, and the mobs whose own timer
 *       shows in game: chickens (eggs), zombie villagers (curing), piglins and hoglins
 *       (zombification), armadillos (scutes).
 *   <li>On skipped ticks the despawn timer and babies' growth still advance, so mobs
 *       despawn and grow up as fast as before.
 * </ul>
 */
public final class ActivationRange {
    public static final int INTERVAL = 4;
    private static final int RANGE = TandemOptimizations.ACTIVATION_RANGE;
    private static final float BOSS_HEALTH = 100.0F;
    private static final boolean DEBUG = Boolean.getBoolean("tandem.optimizations.debug");
    private static boolean broken;
    private static final Map<Class<?>, Boolean> THROTTLEABLE = new ConcurrentHashMap<>();
    private static long skipped;
    private static long ticked;
    private static long nextLog;

    private ActivationRange() {}

    /**
     * Called by the world for every entity tick: false skips this tick. A Minecraft
     * version where a method used here is missing turns the activation range off instead
     * of crashing the game.
     */
    public static boolean shouldTick(class_3218 world, class_1297 entity) {
        if (RANGE <= 0 || broken || entity.field_6012 % INTERVAL == 0 || !(entity instanceof class_1308 mob)) {
            return true;
        }
        try {
            if (!throttleable(mob) || busy(mob) || nearPlayer(world, mob)) {
                if (DEBUG) count(false);
                return true;
            }
            idleTick(mob);
        } catch (LinkageError e) {
            broken = true;
            TandemOptimizations.LOG.warn("Activation range turned off: this Minecraft version is not supported", e);
            return true;
        }
        if (DEBUG) count(true);
        return false;
    }

    /** Per entity class, computed once: a vanilla mob without a timer that shows. */
    private static boolean throttleable(class_1308 mob) {
        return THROTTLEABLE.computeIfAbsent(mob.getClass(), c -> c.getName().startsWith("net.minecraft.")
                && !(mob instanceof class_3988 || mob instanceof class_3763
                        || mob instanceof class_1428 || mob instanceof class_1641
                        || mob instanceof class_5418 || mob instanceof class_4760
                        || mob instanceof class_1510 || mob instanceof class_1528
                        || mob instanceof class_7260 || mob instanceof class_1550
                        || "entity.minecraft.armadillo".equals(mob.method_5864().method_5882())));
    }

    private static boolean busy(class_1308 mob) {
        return mob.field_6235 > 0
                || !mob.method_24828()
                || mob.method_5799()
                || mob.method_5771()
                || mob.method_5809()
                || mob.method_5968() != null
                || mob.method_5765()
                || mob.method_5782()
                || !mob.method_6026().isEmpty()
                || mob instanceof class_1321 tameable && tameable.method_6181()
                || mob instanceof class_1429 animal && animal.method_6479()
                || mob.method_6063() >= BOSS_HEALTH;
    }

    private static boolean nearPlayer(class_3218 world, class_1308 mob) {
        List<? extends class_1657> players = world.method_18456();
        double x = mob.method_23317();
        double z = mob.method_23321();
        for (int i = 0, n = players.size(); i < n; i++) {
            class_1657 player = players.get(i);
            if (Math.abs(player.method_23317() - x) <= RANGE && Math.abs(player.method_23321() - z) <= RANGE) {
                return true;
            }
        }
        return false;
    }

    /** What vanilla would have advanced during the skipped tick (MobEntity, PassiveEntity). */
    private static void idleTick(class_1308 mob) {
        if (!mob.method_5987()) {
            mob.method_16826(mob.method_6131() + 1);
        }
        if (mob instanceof class_1296 passive && passive.method_5805()) {
            int age = passive.method_5618();
            if (age < 0) {
                passive.method_5614(age + 1);
            } else if (age > 0) {
                passive.method_5614(age - 1);
            }
        }
    }

    /** Debug only: share of the checked mob ticks that were skipped, every 10 s. */
    private static void count(boolean skip) {
        if (skip) skipped++;
        else ticked++;
        long now = System.nanoTime();
        if (now > nextLog) {
            if (nextLog != 0) {
                TandemOptimizations.LOG.info("activation range: {} mob ticks skipped, {} kept", skipped, ticked);
            }
            skipped = 0;
            ticked = 0;
            nextLog = now + 10_000_000_000L;
        }
    }
}
