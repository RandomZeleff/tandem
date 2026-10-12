package tandem.optimizations.mixin;

import java.util.function.BooleanSupplier;

import net.minecraft.server.MinecraftServer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import tandem.optimizations.WorldHealth;

/** MinecraftServer.tick: times each tick for the load indicator (see WorldHealth). */
@Mixin(MinecraftServer.class)
public abstract class MinecraftServerMixin {
    @Inject(method = "method_3748", remap = false, at = @At("HEAD"))
    private void tandem$tickStart(BooleanSupplier hasTimeLeft, CallbackInfo ci) {
        WorldHealth.begin();
    }

    @Inject(method = "method_3748", remap = false, at = @At("RETURN"))
    private void tandem$tickEnd(BooleanSupplier hasTimeLeft, CallbackInfo ci) {
        WorldHealth.end((MinecraftServer) (Object) this);
    }
}
