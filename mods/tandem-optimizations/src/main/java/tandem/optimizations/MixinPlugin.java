package tandem.optimizations;

import java.util.ArrayList;
import java.util.List;
import java.util.Set;

import net.fabricmc.loader.api.FabricLoader;
import org.objectweb.asm.tree.ClassNode;
import org.spongepowered.asm.mixin.extensibility.IMixinConfigPlugin;
import org.spongepowered.asm.mixin.extensibility.IMixinInfo;

/**
 * Adds the library fixes only when their library is installed, so Mixin does not look
 * for missing classes (and warn about them) in every other modpack.
 */
public final class MixinPlugin implements IMixinConfigPlugin {
    @Override
    public void onLoad(String mixinPackage) {}

    @Override
    public String getRefMapperConfig() {
        return null;
    }

    @Override
    public boolean shouldApplyMixin(String targetClassName, String mixinClassName) {
        return true;
    }

    @Override
    public void acceptTargets(Set<String> myTargets, Set<String> otherTargets) {}

    @Override
    public List<String> getMixins() {
        List<String> mixins = new ArrayList<>();
        FabricLoader loader = FabricLoader.getInstance();
        if (loader.isModLoaded("architectury")) mixins.add("ArchitecturyEventFactoryMixin");
        if (loader.isModLoaded("balm")) mixins.add("BalmConfigReflectionMixin");
        // Read here, not through WorldHealth: loading that class now would load
        // MinecraftServer before its Mixins are applied.
        if (!"false".equalsIgnoreCase(System.getProperty("tandem.optimizations.health"))) {
            mixins.add("MinecraftServerMixin");
        }
        return mixins;
    }

    @Override
    public void preApply(String targetClassName, ClassNode targetClass, String mixinClassName, IMixinInfo mixinInfo) {}

    @Override
    public void postApply(String targetClassName, ClassNode targetClass, String mixinClassName, IMixinInfo mixinInfo) {}
}
