package dev.incular

import android.app.Activity
import android.content.pm.PackageManager
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.View
import android.view.WindowInsets

/**
 * Full-screen host for an Incular application.
 *
 * Loads the app's native library (manifest meta-data `dev.incular.lib_name`,
 * default `incular_app`), draws edge to edge into a [SurfaceView], and forwards
 * surface, touch, inset and lifecycle callbacks to the Rust engine. All policy
 * lives in Rust; this class only translates.
 */
open class IncularActivity : Activity(), SurfaceHolder.Callback {
    private lateinit var surfaceView: SurfaceView
    private var surfaceAttached = false
    private var started = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        System.loadLibrary(libraryName())
        drawEdgeToEdge()
        surfaceView = SurfaceView(this)
        surfaceView.holder.addCallback(this)
        surfaceView.setOnTouchListener { _, event -> forwardTouch(event) }
        surfaceView.setOnApplyWindowInsetsListener { view, insets -> forwardInsets(insets); view.onApplyWindowInsets(insets) }
        setContentView(surfaceView)
        started = IncularNative.nativeCreate()
        if (!started) finish()
    }

    private fun libraryName(): String {
        val info = packageManager.getActivityInfo(componentName, PackageManager.GET_META_DATA)
        return info.metaData?.getString("dev.incular.lib_name") ?: "incular_app"
    }

    @Suppress("DEPRECATION")
    private fun drawEdgeToEdge() {
        window.statusBarColor = Color.TRANSPARENT
        window.navigationBarColor = Color.TRANSPARENT
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            window.setDecorFitsSystemWindows(false)
        } else {
            window.decorView.systemUiVisibility = View.SYSTEM_UI_FLAG_LAYOUT_STABLE or
                View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN or
                View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
        }
    }

    private val density: Float get() = resources.displayMetrics.density

    override fun surfaceCreated(holder: SurfaceHolder) {}

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        if (!started) return
        if (surfaceAttached) {
            IncularNative.nativeSurfaceChanged(width, height, density)
        } else {
            surfaceAttached = true
            IncularNative.nativeSurfaceCreated(holder.surface, width, height, density)
        }
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        if (!started || !surfaceAttached) return
        surfaceAttached = false
        IncularNative.nativeSurfaceDestroyed()
    }

    private fun forwardTouch(event: MotionEvent): Boolean {
        if (!started) return false
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> send(event, event.actionIndex, 0)
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> send(event, event.actionIndex, 2)
            MotionEvent.ACTION_MOVE -> {
                for (h in 0 until event.historySize) {
                    for (i in 0 until event.pointerCount) {
                        IncularNative.nativeTouch(
                            event.getPointerId(i), 1,
                            event.getHistoricalX(i, h), event.getHistoricalY(i, h),
                            event.getHistoricalPressure(i, h), event.getToolType(i),
                        )
                    }
                }
                for (i in 0 until event.pointerCount) send(event, i, 1)
            }
            MotionEvent.ACTION_CANCEL -> for (i in 0 until event.pointerCount) send(event, i, 3)
            else -> return false
        }
        return true
    }

    private fun send(event: MotionEvent, index: Int, action: Int) {
        IncularNative.nativeTouch(
            event.getPointerId(index), action,
            event.getX(index), event.getY(index),
            event.getPressure(index), event.getToolType(index),
        )
    }

    @Suppress("DEPRECATION")
    private fun forwardInsets(insets: WindowInsets) {
        if (!started) return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val bars = insets.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout())
            val ime = insets.getInsets(WindowInsets.Type.ime())
            IncularNative.nativeInsets(bars.left, bars.top, bars.right, bars.bottom, ime.bottom)
        } else {
            IncularNative.nativeInsets(
                insets.systemWindowInsetLeft, insets.systemWindowInsetTop,
                insets.systemWindowInsetRight, insets.stableInsetBottom,
                (insets.systemWindowInsetBottom - insets.stableInsetBottom).coerceAtLeast(0),
            )
        }
    }

    override fun onStart() {
        super.onStart()
        if (started) IncularNative.nativeLifecycle(3)
    }

    override fun onResume() {
        super.onResume()
        if (started) IncularNative.nativeLifecycle(0)
    }

    override fun onPause() {
        if (started) IncularNative.nativeLifecycle(1)
        super.onPause()
    }

    override fun onStop() {
        if (started) IncularNative.nativeLifecycle(2)
        super.onStop()
    }
}
