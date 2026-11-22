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
