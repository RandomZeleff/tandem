package tandem.optimizations.mixin;

import java.util.concurrent.ConcurrentHashMap;

import net.minecraft.class_2960;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Pseudo;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
import tandem.optimizations.TandemOptimizations;

/**
 * Balm's ConfigReflection.getIdentifier reads an annotation and builds an Identifier on
 * every call; Balm mods call it for every config access (Forgiving Void: every entity,
 * every tick). The answer never changes for a class: cache it.
 */
@Pseudo
@Mixin(targets = "net.blay09.mods.balm.api.network.ConfigReflection")
public abstract class BalmConfigReflectionMixin {
    private static final ConcurrentHashMap<Class<?>, class_2960> TANDEM$IDS = new ConcurrentHashMap<>();

    @Inject(method = "getIdentifier", at = @At("HEAD"), cancellable = true, remap = false)
    private static void tandem$cached(Class<?> type, CallbackInfoReturnable<class_2960> cir) {
        if (!TandemOptimizations.LIBRARIES) return;
        class_2960 id = TANDEM$IDS.get(type);
        if (id != null) cir.setReturnValue(id);
    }

    @Inject(method = "getIdentifier", at = @At("RETURN"), remap = false)
    private static void tandem$store(Class<?> type, CallbackInfoReturnable<class_2960> cir) {
        if (TandemOptimizations.LIBRARIES && cir.getReturnValue() != null) TANDEM$IDS.putIfAbsent(type, cir.getReturnValue());
    }
}
