package tandem.optimizations;

import net.fabricmc.api.ModInitializer;
import net.fabricmc.loader.api.FabricLoader;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Optimizations the Tandem launcher adds at launch (`-Dfabric.addMods`), never written
 * into the instance. Each one only removes work that has no effect in game, or is
 * switched off from Tandem. Settings come as system properties set by Tandem:
 *
 * <ul>
 *   <li>{@code tandem.optimizations.activationRange}: blocks; mobs farther than this
 *       from every player tick less often. 0 turns it off. Default 32.
 *   <li>{@code tandem.optimizations.spawnGroups}: skip spawn groups where nothing can
 *       spawn. Default true.
 *   <li>{@code tandem.optimizations.libraries}: faster Architectury and Balm calls.
 *       Default true.
 *   <li>{@code tandem.optimizations.health}: tick times in the log while the world is
 *       shared, for Tandem's load indicator. Default true.
 * </ul>
 */
public final class TandemOptimizations implements ModInitializer {
    public static final Logger LOG = LoggerFactory.getLogger("Tandem Optimizations");

    public static final int ACTIVATION_RANGE = intProperty("tandem.optimizations.activationRange", 32);
    public static final boolean SPAWN_GROUPS = boolProperty("tandem.optimizations.spawnGroups");
    public static final boolean LIBRARIES = boolProperty("tandem.optimizations.libraries");

    @Override
    public void onInitialize() {
        String version = FabricLoader.getInstance().getModContainer("tandem_optimizations")
                .map(c -> c.getMetadata().getVersion().getFriendlyString()).orElse("?");
        LOG.info("Tandem Optimizations {} active: activation range {}, spawn groups {}, libraries {}, world health {}",
                version, ACTIVATION_RANGE > 0 ? ACTIVATION_RANGE + " blocks" : "off",
                SPAWN_GROUPS ? "on" : "off", LIBRARIES ? "on" : "off", WorldHealth.ENABLED ? "on" : "off");
    }

    private static boolean boolProperty(String key) {
        return !"false".equalsIgnoreCase(System.getProperty(key));
    }

    private static int intProperty(String key, int fallback) {
        try {
            return Math.max(0, Integer.parseInt(System.getProperty(key, String.valueOf(fallback))));
        } catch (NumberFormatException e) {
            return fallback;
        }
    }
}
