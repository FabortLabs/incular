package dev.incular

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.view.Surface

/**
 * JNI entry points implemented by `incular-android` (`src/shell.rs`), and the static methods
 * Rust calls back (`src/shell/text_input.rs`). Main thread only. Text offsets are UTF-16.
 */
internal object IncularNative {
    /** The view the soft keyboard edits, while an activity shows one. */
    internal var input: IncularInputView? = null
    /** The activity that owns the system bars, while one is shown. */
    internal var activity: IncularActivity? = null
    /** Icon brightness values `setSystemUiOverlayStyle` takes: unset, light icons, dark icons. */
    internal const val ICON_UNSET = -1
    internal const val ICON_DARK = 1
    private var context: Context? = null

    /**
     * Starts the app. [context] is the application context, published to Rust through
     * `ndk-context`; [filesDir] is `Context.getFilesDir()`, the app's private data root.
     */
    fun create(context: Context, filesDir: String): Boolean {
        this.context = context
        return nativeCreate(context, filesDir)
    }

    @JvmStatic private external fun nativeCreate(context: Context, filesDir: String): Boolean

    @JvmStatic external fun nativeSurfaceCreated(surface: Surface, width: Int, height: Int, density: Float)

    @JvmStatic external fun nativeSurfaceChanged(width: Int, height: Int, density: Float)

    @JvmStatic external fun nativeSurfaceDestroyed()

    /**
     * `action`: 0 down, 1 move, 2 up, 3 cancel, 4 hover enter, 5 hover exit. Physical pixels.
     * [buttons] is the pressed-button mask and [changedButton] the button that went down or up
     * (0 when none), both in Incular's pointer-button bits. [ageNanos] is how long ago the
     * platform recorded the sample. [tilt] and [orientation] are stylus axes in radians, NaN
     * when the tool is not a stylus.
     */
    @JvmStatic external fun nativeTouch(
        pointer: Int,
        device: Int,
        action: Int,
        x: Float,
        y: Float,
        pressure: Float,
        toolType: Int,
        buttons: Int,
        changedButton: Int,
        ageNanos: Long,
        tilt: Float,
        orientation: Float,
    )

    /** A wheel or trackpad scroll at ([x], [y]); [dx] and [dy] are physical pixels, positive down and right. */
    @JvmStatic external fun nativeScroll(x: Float, y: Float, dx: Float, dy: Float, ageNanos: Long)

    /** Physical-pixel system bar/cutout insets, then the soft keyboard's bottom inset. */
    @JvmStatic external fun nativeInsets(left: Int, top: Int, right: Int, bottom: Int, imeBottom: Int)

    /**
     * The system settings Flutter reports: dark mode, font scale, bold text, reduced motion (animator
     * transition scale 0) and the locale tags in preference order, joined with commas.
     */
    @JvmStatic external fun nativeSettings(dark: Boolean, textScale: Float, boldText: Boolean, reducedMotion: Boolean, locales: String)

    /** 0 resumed, 1 paused, 2 stopped. */
    @JvmStatic external fun nativeLifecycle(state: Int)

    /** The keyboard's mirror after an edit; a composing start of -1 means none. */
    @JvmStatic external fun nativeEditingState(text: String, selStart: Int, selEnd: Int, compStart: Int, compEnd: Int)

    /** The keyboard's action key, an `EditorInfo.IME_ACTION_*` value. */
    @JvmStatic external fun nativeEditorAction(action: Int)

    /** A hardware key; returns whether Incular took it. */
    @JvmStatic external fun nativeKey(down: Boolean, keyCode: Int, unicode: Int, metaState: Int, repeat: Boolean): Boolean

    /** The back button; returns false when the app did not use it and the activity should finish. */
    @JvmStatic external fun nativeBack(): Boolean

    /** Applies the style Rust asked for; see `IncularActivity.applySystemUiOverlayStyle`. */
    @JvmStatic fun setSystemUiOverlayStyle(statusIcons: Int, navIcons: Int, statusColor: Int, navColor: Int, flags: Int) {
        activity?.applySystemUiOverlayStyle(statusIcons, navIcons, statusColor, navColor, flags)
    }

    @JvmStatic fun showTextInput(inputType: Int, imeOptions: Int, text: String, selStart: Int, selEnd: Int, compStart: Int, compEnd: Int) {
        input?.attach(inputType, imeOptions, text, selStart, selEnd, compStart, compEnd)
    }

    @JvmStatic fun updateTextInput(text: String, selStart: Int, selEnd: Int, compStart: Int, compEnd: Int) {
        input?.update(text, selStart, selEnd, compStart, compEnd)
    }

    @JvmStatic fun showSoftInput() {
        input?.showKeyboard()
    }

    @JvmStatic fun hideTextInput(detach: Boolean) {
        input?.hide(detach)
    }

    private fun clipboard(): ClipboardManager? = context?.getSystemService(ClipboardManager::class.java)

    @JvmStatic fun clipboardText(): String? {
        val clip = clipboard()?.primaryClip ?: return null
        if (clip.itemCount == 0) return null
        return clip.getItemAt(0).coerceToText(context).toString()
    }

    @JvmStatic fun setClipboardText(text: String) {
        clipboard()?.setPrimaryClip(ClipData.newPlainText("text", text))
    }

    /** `Clipboard.hasStrings`: checks without reading, which Android 12+ would announce. */
    @JvmStatic fun clipboardHasText(): Boolean {
        val clipboard = clipboard() ?: return false
        return clipboard.hasPrimaryClip() && clipboard.primaryClipDescription?.hasMimeType("text/*") == true
    }
}
