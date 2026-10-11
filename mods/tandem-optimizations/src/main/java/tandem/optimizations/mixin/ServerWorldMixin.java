package tandem.optimizations.mixin;

import com.llamalad7.mixinextras.injector.WrapWithCondition;
import net.minecraft.class_1297;
import net.minecraft.class_3218;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import tandem.optimizations.ActivationRange;

/** ServerWorld.tickEntity: `entity.tick()` only when the activation range allows it. */
@Mixin(class_3218.class)
public abstract class ServerWorldMixin {
    @WrapWithCondition(method = "method_18762", remap = false,
            at = @At(value = "INVOKE", target = "Lnet/minecraft/class_1297;method_5773()V", remap = false))
    private boolean tandem$activationRange(class_1297 entity) {
        return ActivationRange.shouldTick((class_3218) (Object) this, entity);
    }
}
