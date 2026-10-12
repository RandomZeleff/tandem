package tandem.optimizations;

import java.util.Locale;

import net.minecraft.server.MinecraftServer;

/**
 * How long the server's ticks take, for the load indicator Tandem shows the host. Every
 * 100 ticks (5 s when the world keeps up) one line goes to the log, read by Tandem:
 * {@code [Tandem] world health: mspt=12.3 max=40.1 players=3}. Only while the world is
 * shared (a dedicated server, or a world opened to LAN), so solo logs stay quiet.
 *
 * <p>{@code tandem.optimizations.health=false} turns it off.
 */
public final class WorldHealth {
    public static final boolean ENABLED =
            !"false".equalsIgnoreCase(System.getProperty("tandem.optimizations.health"));
    private static final int WINDOW = 100;

    private static boolean broken;
    private static long started;
    private static long sum;
    private static long max;
    private static int ticks;

    private WorldHealth() {}

    public static void begin() {
        started = System.nanoTime();
    }

    public static void end(MinecraftServer server) {
        if (broken || started == 0) return;
        long took = System.nanoTime() - started;
        sum += took;
        max = Math.max(max, took);
        if (++ticks < WINDOW) return;
        try {
            if (server.method_3816() || server.method_3860()) {
                TandemOptimizations.LOG.info(String.format(Locale.ROOT,
                        "[Tandem] world health: mspt=%.1f max=%.1f players=%d",
                        sum / (double) ticks / 1e6, max / 1e6, server.method_3788()));
            }
        } catch (LinkageError e) {
            // A method this version does not have: no report rather than a crash.
            broken = true;
            TandemOptimizations.LOG.warn("world health report off: {}", e.toString());
        }
        sum = 0;
        max = 0;
        ticks = 0;
    }
}
