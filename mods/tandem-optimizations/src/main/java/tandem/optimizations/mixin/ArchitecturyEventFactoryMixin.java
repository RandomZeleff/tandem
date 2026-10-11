package tandem.optimizations.mixin;

import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Pseudo;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
import tandem.optimizations.TandemOptimizations;

/**
 * Architectury's EventFactory.invokeMethod does lookup().unreflect(method).bindTo(listener)
 * .invokeWithArguments(args) for every listener of every event, every time (server tick,
 * entity tick...). Method.invoke caches its accessor per Method: same call, a fraction of
 * the cost. Falls back to the original code if access is refused.
 */
@Pseudo
@Mixin(targets = "dev.architectury.event.EventFactory")
public abstract class ArchitecturyEventFactoryMixin {
    @Inject(method = "invokeMethod", at = @At("HEAD"), cancellable = true, remap = false)
    private static void tandem$fastInvoke(Object listener, Method method, Object[] args,
            CallbackInfoReturnable<Object> cir) {
        if (!TandemOptimizations.LIBRARIES) return;
        try {
            cir.setReturnValue(method.invoke(listener, args));
        } catch (InvocationTargetException e) {
            throw tandem$sneaky(e.getCause());
        } catch (IllegalAccessException | IllegalArgumentException e) {
            // let the original code run
        }
    }

    @SuppressWarnings("unchecked")
    private static <E extends Throwable> RuntimeException tandem$sneaky(Throwable t) throws E {
        throw (E) t;
    }
}
