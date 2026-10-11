package tandem.optimizations.mixin;

import net.minecraft.class_1311;
import net.minecraft.class_1948;
import net.minecraft.class_2818;
import net.minecraft.class_3218;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import tandem.optimizations.DeadSpawnGroups;
import tandem.optimizations.TandemOptimizations;

/** SpawnHelper.spawnEntitiesInChunk: skip groups where nothing can ever spawn. */
@Mixin(class_1948.class)
public abstract class SpawnHelperMixin {
    @Inject(method = "method_8663", at = @At("HEAD"), cancellable = true, remap = false)
    private static void tandem$skipDeadGroup(class_1311 group, class_3218 world, class_2818 chunk,
            class_1948.class_5261 checker, class_1948.class_5259 runner, CallbackInfo ci) {
        if (TandemOptimizations.SPAWN_GROUPS && DeadSpawnGroups.isDead(group, world)) ci.cancel();
    }
}
