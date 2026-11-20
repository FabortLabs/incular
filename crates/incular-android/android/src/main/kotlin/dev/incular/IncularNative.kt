package dev.incular

import android.view.Surface

/** JNI entry points implemented by `incular-android` (`src/shell.rs`). Main thread only. */
internal object IncularNative {
    @JvmStatic external fun nativeCreate(): Boolean

    @JvmStatic external fun nativeSurfaceCreated(surface: Surface, width: Int, height: Int, density: Float)

    @JvmStatic external fun nativeSurfaceChanged(width: Int, height: Int, density: Float)

    @JvmStatic external fun nativeSurfaceDestroyed()

    /** `action`: 0 down, 1 move, 2 up, 3 cancel. Physical pixels. */
    @JvmStatic external fun nativeTouch(
        pointer: Int,
        action: Int,
        x: Float,
        y: Float,
        pressure: Float,
        toolType: Int,
    )

    /** Physical-pixel system bar/cutout insets, then the soft keyboard's bottom inset. */
    @JvmStatic external fun nativeInsets(left: Int, top: Int, right: Int, bottom: Int, imeBottom: Int)

    /** 0 resumed, 1 paused, 2 stopped, 3 started. */
    @JvmStatic external fun nativeLifecycle(state: Int)
}
