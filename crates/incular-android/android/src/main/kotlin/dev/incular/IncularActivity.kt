package dev.incular

import android.app.Activity
import android.content.pm.PackageManager
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.content.res.Configuration
import android.provider.Settings
import android.util.SparseIntArray
import android.view.InputDevice
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.ViewConfiguration
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
    private val mouseButtons = SparseIntArray()

    private companion object {
        /** FontWeight.Bold - FontWeight.Normal, as Flutter's AccessibilityBridge defines it. */
        const val BOLD_TEXT_WEIGHT_ADJUSTMENT = 300

        /** Incular's pointer-button bits, the same values as incular-core's `*_POINTER_BUTTON` constants. */
        const val BUTTON_PRIMARY = 1
        const val BUTTON_SECONDARY = 2
        const val BUTTON_TERTIARY = 4
        const val BUTTON_BACK = 8
        const val BUTTON_FORWARD = 16

        /** Sample actions, the codes `IncularNative.nativeTouch` documents. */
        const val PHASE_DOWN = 0
        const val PHASE_MOVE = 1
        const val PHASE_UP = 2
        const val PHASE_CANCEL = 3
        const val PHASE_HOVER_ENTER = 4
        const val PHASE_HOVER_EXIT = 5

        /** Passed as a history index to mean the event's current sample rather than a historical one. */
        const val CURRENT_SAMPLE = -1
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
        surfaceView.setOnHoverListener { _, event -> forwardHover(event) }
        surfaceView.setOnGenericMotionListener { _, event -> forwardGenericMotion(event) }
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

    /** Touch contacts, stylus contacts and mouse drags. */
    private fun forwardTouch(event: MotionEvent): Boolean {
        if (!started) return false
        val index = event.actionIndex
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN,
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                if (event.getToolType(index) == MotionEvent.TOOL_TYPE_MOUSE) {
                    forwardMouseButtons(event, index)
                } else {
                    val down = event.actionMasked == MotionEvent.ACTION_DOWN ||
                        event.actionMasked == MotionEvent.ACTION_POINTER_DOWN
                    send(event, index, if (down) PHASE_DOWN else PHASE_UP, changed = BUTTON_PRIMARY)
                }
            }
            MotionEvent.ACTION_MOVE -> {
                // Historical samples come first, each stamped with the time it was recorded.
                for (h in 0 until event.historySize) {
                    for (i in 0 until event.pointerCount) send(event, i, PHASE_MOVE, h)
                }
                for (i in 0 until event.pointerCount) send(event, i, PHASE_MOVE)
            }
            MotionEvent.ACTION_CANCEL -> {
                mouseButtons.delete(event.deviceId)
                for (i in 0 until event.pointerCount) send(event, i, PHASE_CANCEL)
            }
            else -> return false
        }
        return true
    }

    /** Hover follows Android's separate hover dispatch path. */
    private fun forwardHover(event: MotionEvent): Boolean {
        if (!started) return false
        when (event.actionMasked) {
            MotionEvent.ACTION_HOVER_ENTER -> send(event, 0, PHASE_HOVER_ENTER)
            MotionEvent.ACTION_HOVER_MOVE -> {
                for (h in 0 until event.historySize) send(event, 0, PHASE_MOVE, h)
                send(event, 0, PHASE_MOVE)
            }
            MotionEvent.ACTION_HOVER_EXIT -> send(event, 0, PHASE_HOVER_EXIT)
            else -> return false
        }
        return true
    }

    private fun forwardGenericMotion(event: MotionEvent): Boolean {
        if (!started || !event.isFromSource(InputDevice.SOURCE_CLASS_POINTER)) return false
        when (event.actionMasked) {
            MotionEvent.ACTION_BUTTON_PRESS, MotionEvent.ACTION_BUTTON_RELEASE -> {
                val index = event.actionIndex
                if (event.getToolType(index) != MotionEvent.TOOL_TYPE_MOUSE) return false
                forwardMouseButtons(event, index)
            }
            MotionEvent.ACTION_SCROLL -> {
                // Positive axis values scroll up and left; Incular's scroll deltas point down and right.
                val config = ViewConfiguration.get(this)
                IncularNative.nativeScroll(
                    event.x, event.y,
                    -event.getAxisValue(MotionEvent.AXIS_HSCROLL) * config.scaledHorizontalScrollFactor,
                    -event.getAxisValue(MotionEvent.AXIS_VSCROLL) * config.scaledVerticalScrollFactor,
                    ageNanos(eventTimeNanos(event, CURRENT_SAMPLE)),
                )
            }
            else -> return false
        }
        return true
    }

    /** Android sends both touch and generic events for the first press and last release. */
    private fun forwardMouseButtons(event: MotionEvent, index: Int) {
        val device = event.deviceId
        val previous = mouseButtons.get(device)
        val buttons = buttonMask(event.buttonState)
        if (buttons == 0) mouseButtons.delete(device) else mouseButtons.put(device, buttons)
        var changed = previous xor buttons
        while (changed != 0) {
            val button = Integer.lowestOneBit(changed)
            val phase = if (buttons and button != 0) PHASE_DOWN else PHASE_UP
            send(event, index, phase, changed = button)
            changed = changed xor button
        }
    }

    /**
     * Sends pointer [index] of [event] as sample [history] (or the current sample for
     * [CURRENT_SAMPLE]) with a `PHASE_*` action.
     */
    private fun send(event: MotionEvent, index: Int, action: Int, history: Int = CURRENT_SAMPLE, changed: Int = 0) {
        val current = history == CURRENT_SAMPLE
        val tool = event.getToolType(index)
        val stylus = tool == MotionEvent.TOOL_TYPE_STYLUS || tool == MotionEvent.TOOL_TYPE_ERASER
        val hovering = event.actionMasked == MotionEvent.ACTION_HOVER_ENTER ||
            event.actionMasked == MotionEvent.ACTION_HOVER_MOVE ||
            event.actionMasked == MotionEvent.ACTION_HOVER_EXIT
        val buttons = when {
            action == PHASE_CANCEL || action == PHASE_HOVER_EXIT -> 0
            tool == MotionEvent.TOOL_TYPE_MOUSE -> buttonMask(event.buttonState)
            // Only the tip presses primary; a stylus can report its barrel button in hover as well.
            else -> (if (!hovering && (action == PHASE_DOWN || action == PHASE_MOVE)) BUTTON_PRIMARY else 0) or
                (if (stylus) buttonMask(event.buttonState) else 0)
        }
        IncularNative.nativeTouch(
            event.getPointerId(index), event.deviceId, action,
            if (current) event.getX(index) else event.getHistoricalX(index, history),
            if (current) event.getY(index) else event.getHistoricalY(index, history),
            if (current) event.getPressure(index) else event.getHistoricalPressure(index, history),
            tool, buttons, changed,
            ageNanos(eventTimeNanos(event, history)),
            if (stylus) axis(event, MotionEvent.AXIS_TILT, index, history) else Float.NaN,
            if (stylus) axis(event, MotionEvent.AXIS_ORIENTATION, index, history) else Float.NaN,
        )
    }

    private fun axis(event: MotionEvent, axis: Int, index: Int, history: Int): Float =
        if (history == CURRENT_SAMPLE) event.getAxisValue(axis, index)
        else event.getHistoricalAxisValue(axis, index, history)

    /**
     * When a sample was recorded, on the `System.nanoTime` clock: both are CLOCK_MONOTONIC.
     * Before API 34 the event times are milliseconds.
     */
    private fun eventTimeNanos(event: MotionEvent, history: Int): Long {
        if (Build.VERSION.SDK_INT >= 34) {
            return if (history == CURRENT_SAMPLE) event.eventTimeNanos else event.getHistoricalEventTimeNanos(history)
        }
        val millis = if (history == CURRENT_SAMPLE) event.eventTime else event.getHistoricalEventTime(history)
        return millis * 1_000_000L
    }

    /** How long ago a sample recorded at [eventNanos] was taken, never negative. */
    private fun ageNanos(eventNanos: Long): Long = (System.nanoTime() - eventNanos).coerceAtLeast(0L)

    /** Incular's pointer-button mask for Android's `MotionEvent.BUTTON_*` bits, as Flutter maps them. */
    private fun buttonMask(androidButtons: Int): Int {
        var mask = 0
        if (androidButtons and MotionEvent.BUTTON_PRIMARY != 0) mask = mask or BUTTON_PRIMARY
        if (androidButtons and (MotionEvent.BUTTON_SECONDARY or MotionEvent.BUTTON_STYLUS_PRIMARY) != 0) {
            mask = mask or BUTTON_SECONDARY
        }
        if (androidButtons and (MotionEvent.BUTTON_TERTIARY or MotionEvent.BUTTON_STYLUS_SECONDARY) != 0) {
            mask = mask or BUTTON_TERTIARY
        }
        if (androidButtons and MotionEvent.BUTTON_BACK != 0) mask = mask or BUTTON_BACK
        if (androidButtons and MotionEvent.BUTTON_FORWARD != 0) mask = mask or BUTTON_FORWARD
        return mask
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
