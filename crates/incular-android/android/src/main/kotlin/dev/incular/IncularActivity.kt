package dev.incular

import android.app.Activity
import android.content.pm.PackageManager
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.content.res.Configuration
import android.provider.Settings
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.window.OnBackInvokedCallback
import android.window.OnBackInvokedDispatcher
import android.view.View
import android.view.WindowInsets
import android.view.WindowInsetsController

/**
 * Full-screen host for an Incular application.
 *
 * Loads the app's native library (manifest meta-data `dev.incular.lib_name`,
 * default `incular_app`), draws edge to edge into a [SurfaceView], and forwards
 * surface, touch, inset and lifecycle callbacks to the Rust engine. All policy
 * lives in Rust; this class only translates.
 */
open class IncularActivity : Activity(), SurfaceHolder.Callback {
    private lateinit var surfaceView: IncularInputView
    private var surfaceAttached = false
    private var started = false

    private companion object {
        /** FontWeight.Bold - FontWeight.Normal, as Flutter's AccessibilityBridge defines it. */
        const val BOLD_TEXT_WEIGHT_ADJUSTMENT = 300
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        System.loadLibrary(libraryName())
        drawEdgeToEdge()
        surfaceView = IncularInputView(this)
        IncularNative.input = surfaceView
        IncularNative.activity = this
        surfaceView.holder.addCallback(this)
        surfaceView.setOnTouchListener { _, event -> forwardTouch(event) }
        surfaceView.setOnApplyWindowInsetsListener { view, insets -> forwardInsets(insets); view.onApplyWindowInsets(insets) }
        setContentView(surfaceView)
        started = IncularNative.create(applicationContext, filesDir.absolutePath)
        if (!started) finish() else {
            registerBack()
            sendSettings(resources.configuration)
        }
    }

    /**
     * The back button and the back gesture go to Rust first (Flutter's `popRoute`): it closes
     * a popup or pops a route. When nothing used them the activity finishes, as Flutter's
     * `SystemNavigator.pop` does. Android 13+ reports them to [OnBackInvokedCallback] (the
     * manifest opts in); older versions call [onBackPressed]. The soft keyboard takes the
     * back press first while it is open.
     */
    private fun registerBack() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            onBackInvokedDispatcher.registerOnBackInvokedCallback(
                OnBackInvokedDispatcher.PRIORITY_DEFAULT,
                OnBackInvokedCallback { if (!IncularNative.nativeBack()) finish() },
            )
        }
    }

    @Deprecated("Android 13+ uses the OnBackInvokedCallback registered in registerBack")
    @Suppress("DEPRECATION")
    override fun onBackPressed() {
        if (started && IncularNative.nativeBack()) return
        super.onBackPressed()
    }

    override fun onDestroy() {
        if (IncularNative.input === surfaceView) IncularNative.input = null
        if (IncularNative.activity === this) IncularNative.activity = null
        super.onDestroy()
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

    /**
     * Flutter `PlatformPlugin.setSystemChromeSystemUIOverlayStyle`: icon brightness (unset, light or
     * dark) and the bar colours that `flags` names. Colours apply only below API 35, where
     * edge-to-edge forces the bars transparent.
     */
    @Suppress("DEPRECATION")
    internal fun applySystemUiOverlayStyle(statusIcons: Int, navIcons: Int, statusColor: Int, navColor: Int, flags: Int) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val controller: WindowInsetsController = window.insetsController ?: return
            var mask = 0
            var appearance = 0
            if (statusIcons != IncularNative.ICON_UNSET) {
                mask = mask or WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
                if (statusIcons == IncularNative.ICON_DARK) appearance = appearance or WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
            }
            if (navIcons != IncularNative.ICON_UNSET) {
                mask = mask or WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS
                if (navIcons == IncularNative.ICON_DARK) appearance = appearance or WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS
            }
            if (mask != 0) controller.setSystemBarsAppearance(appearance, mask)
        } else {
            var visibility = window.decorView.systemUiVisibility
            visibility = withFlag(visibility, View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR, statusIcons)
            visibility = withFlag(visibility, View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR, navIcons)
            window.decorView.systemUiVisibility = visibility
        }
        if (Build.VERSION.SDK_INT < 35) {
            if ((flags and 1) != 0) window.statusBarColor = statusColor
            if ((flags and 2) != 0) window.navigationBarColor = navColor
        }
    }

    /** Sets or clears [flag] in [visibility] for dark icons, leaving it alone when [icons] is unset. */
    private fun withFlag(visibility: Int, flag: Int, icons: Int): Int = when (icons) {
        IncularNative.ICON_UNSET -> visibility
        IncularNative.ICON_DARK -> visibility or flag
        else -> visibility and flag.inv()
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

    override fun onResume() {
        super.onResume()
        if (started) {
            // Settings.Global animation scale changes do not recreate the activity, so re-read on resume.
            sendSettings(resources.configuration)
            IncularNative.nativeLifecycle(0)
        }
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        if (started) sendSettings(newConfig)
    }

    /**
     * Reports what Flutter reads in `FlutterView.sendUserSettingsToFlutter`, `AccessibilityBridge`
     * and `LocalizationPlugin.sendLocalesToFlutter`: dark mode from the night flag, the font scale,
     * bold text (font weight adjustment of at least 300, API 31+), reduced motion (transition
     * animation scale 0) and the locale tags in order.
     */
    private fun sendSettings(config: Configuration) {
        val dark = (config.uiMode and Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES
        val boldText = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S &&
            config.fontWeightAdjustment != Configuration.FONT_WEIGHT_ADJUSTMENT_UNDEFINED &&
            config.fontWeightAdjustment >= BOLD_TEXT_WEIGHT_ADJUSTMENT
        val transitionScale = Settings.Global.getFloat(contentResolver, Settings.Global.TRANSITION_ANIMATION_SCALE, 1f)
        IncularNative.nativeSettings(
            dark, config.fontScale, boldText, transitionScale == 0f, config.locales.toLanguageTags(),
        )
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
