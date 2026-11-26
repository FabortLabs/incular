package dev.incular

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.icu.text.BreakIterator
import android.os.Build
import android.os.LocaleList
import android.text.Editable
import android.text.InputType
import android.text.Selection
import android.text.Spanned
import android.view.KeyEvent
import android.view.SurfaceView
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager

/**
 * The [SurfaceView] Incular draws into, which is also the text editor the soft keyboard
 * talks to.
 *
 * As in Flutter's `TextInputPlugin`, the Rust engine owns the focused field's value. This
 * view keeps a mirror [Editable] for the input method: [IncularInputConnection] applies the
 * keyboard's edits to it and reports its whole state to Rust, and Rust sends the field's
 * state back ([update]) when the app changed it. Hardware keys go to Rust as key events.
 */
internal class IncularInputView(context: Context) : SurfaceView(context) {
    private val imm = context.getSystemService(InputMethodManager::class.java)
    private var inputType = InputType.TYPE_NULL
    private var imeOptions = 0
    private var languageTags: String? = null
    private var attached = false
    private val editable: Editable = Editable.Factory.getInstance().newEditable("")

    /** The last mirror state Rust knows: pushed from Rust, or reported by the input connection. */
    internal var reported: MirrorState? = null

    init {
        isFocusable = true
        isFocusableInTouchMode = true
    }

    override fun onCheckIsTextEditor(): Boolean = attached

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection? {
        if (!attached) {
            outAttrs.inputType = InputType.TYPE_NULL
            return null
        }
        outAttrs.inputType = inputType
        outAttrs.imeOptions = imeOptions
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            outAttrs.hintLocales = languageTags?.let { LocaleList.forLanguageTags(it) }
        }
        outAttrs.initialSelStart = Selection.getSelectionStart(editable)
        outAttrs.initialSelEnd = Selection.getSelectionEnd(editable)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) outAttrs.setInitialSurroundingText(editable)
        return IncularInputConnection(this, editable, inputType, imeOptions)
    }

    /**
     * Attaches the input method to a field (`TextInput.setClient`). It does not show the
     * keyboard: a following [showKeyboard] does (`TextInput.show`), so a configuration change
     * the user did not ask for never reopens a keyboard they dismissed.
     */
    fun attach(inputType: Int, imeOptions: Int, text: String, selStart: Int, selEnd: Int, compStart: Int, compEnd: Int, languageTags: String?) {
        this.inputType = inputType
        this.imeOptions = imeOptions
        this.languageTags = languageTags
        attached = true
        setMirror(text, selStart, selEnd, compStart, compEnd)
        requestFocus()
        imm.restartInput(this)
    }

    /** The app changed the field (`TextInput.setEditingState`). */
    fun update(text: String, selStart: Int, selEnd: Int, compStart: Int, compEnd: Int) {
        if (!attached) return
        val textChanged = editable.toString() != text
        setMirror(text, selStart, selEnd, compStart, compEnd)
        if (textChanged) {
            // The input method's cached text and composition are stale.
            imm.restartInput(this)
        } else {
            reportSelection()
        }
    }

    fun showKeyboard() {
        if (!attached) return
        requestFocus()
        imm.showSoftInput(this, 0)
    }

    /** Hides the keyboard; [detach] also ends the field's input session (`TextInput.clearClient`). */
    fun hide(detach: Boolean) {
        imm.hideSoftInputFromWindow(windowToken, 0)
        if (detach && attached) {
            attached = false
            languageTags = null
            setMirror("", 0, 0, -1, -1)
            imm.restartInput(this)
        }
    }

    /** Tells the input method where the selection and composition are, as `EditText` does. */
    fun reportSelection() {
        imm.updateSelection(
            this,
            Selection.getSelectionStart(editable),
            Selection.getSelectionEnd(editable),
            BaseInputConnection.getComposingSpanStart(editable),
            BaseInputConnection.getComposingSpanEnd(editable),
        )
    }

    /**
     * Replaces the mirror with the state Rust pushed. That state is what Rust already has, so
     * it becomes the dedupe baseline: an input method edit that lands on it is not echoed back.
     */
    private fun setMirror(text: String, selStart: Int, selEnd: Int, compStart: Int, compEnd: Int) {
        editable.replace(0, editable.length, text)
        val length = editable.length
        Selection.setSelection(editable, selStart.coerceIn(0, length), selEnd.coerceIn(0, length))
        BaseInputConnection.removeComposingSpans(editable)
        if (compStart in 0..<compEnd && compEnd <= length) {
            editable.setSpan(ComposingSpan(), compStart, compEnd, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE or Spanned.SPAN_COMPOSING)
        }
        reported = MirrorState(
            editable.toString(),
            Selection.getSelectionStart(editable),
            Selection.getSelectionEnd(editable),
            BaseInputConnection.getComposingSpanStart(editable),
            BaseInputConnection.getComposingSpanEnd(editable),
        )
    }

    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean =
        forwardKey(event, true) || super.onKeyDown(keyCode, event)

    override fun onKeyUp(keyCode: Int, event: KeyEvent): Boolean =
        forwardKey(event, false) || super.onKeyUp(keyCode, event)

    /** Hardware keys. Back stays with the activity, which asks Rust through `nativeBack`. */
    private fun forwardKey(event: KeyEvent, down: Boolean): Boolean {
        if (event.keyCode == KeyEvent.KEYCODE_BACK) return false
        return IncularNative.nativeKey(down, event.keyCode, event.getUnicodeChar(event.metaState), event.metaState, event.repeatCount > 0)
    }

    private class ComposingSpan
}

/**
 * Flutter's `InputConnectionAdaptor`: the soft keyboard edits the mirror through
 * [BaseInputConnection], and each finished edit (outside a batch) is reported to Rust.
 */
internal class IncularInputConnection(
    private val view: IncularInputView,
    private val mirror: Editable,
    private val inputType: Int,
    private val imeOptions: Int,
) : BaseInputConnection(view, true) {
    private var batchDepth = 0

    override fun getEditable(): Editable = mirror

    override fun beginBatchEdit(): Boolean {
        batchDepth++
        return true
    }

    override fun endBatchEdit(): Boolean {
        batchDepth = (batchDepth - 1).coerceAtLeast(0)
        if (batchDepth == 0) report()
        return batchDepth > 0
    }

    private fun changed(result: Boolean): Boolean {
        if (batchDepth == 0) report()
        return result
    }

    override fun commitText(text: CharSequence?, newCursorPosition: Int) = changed(super.commitText(text, newCursorPosition))
    override fun setComposingText(text: CharSequence?, newCursorPosition: Int) = changed(super.setComposingText(text, newCursorPosition))
    override fun setComposingRegion(start: Int, end: Int) = changed(super.setComposingRegion(start, end))
    override fun finishComposingText() = changed(super.finishComposingText())
    override fun deleteSurroundingText(beforeLength: Int, afterLength: Int) = changed(super.deleteSurroundingText(beforeLength, afterLength))
    override fun deleteSurroundingTextInCodePoints(beforeLength: Int, afterLength: Int) =
        changed(super.deleteSurroundingTextInCodePoints(beforeLength, afterLength))
    override fun setSelection(start: Int, end: Int) = changed(super.setSelection(start, end))

    override fun performEditorAction(actionCode: Int): Boolean {
        IncularNative.nativeEditorAction(actionCode)
        return true
    }

    override fun performContextMenuAction(id: Int): Boolean {
        val start = Selection.getSelectionStart(mirror).coerceAtLeast(0)
        val end = Selection.getSelectionEnd(mirror).coerceAtLeast(0)
        val (from, to) = minOf(start, end) to maxOf(start, end)
        val clipboard = view.context.getSystemService(ClipboardManager::class.java)
        when (id) {
            android.R.id.selectAll -> Selection.setSelection(mirror, 0, mirror.length)
            android.R.id.copy, android.R.id.cut -> {
                if (from == to) return true
                clipboard.setPrimaryClip(ClipData.newPlainText("text", mirror.subSequence(from, to)))
                if (id == android.R.id.cut) mirror.delete(from, to)
            }
            android.R.id.paste -> {
                val clip = clipboard.primaryClip ?: return true
                if (clip.itemCount == 0) return true
                mirror.replace(from, to, clip.getItemAt(0).coerceToText(view.context))
            }
            else -> return false
        }
        return changed(true)
    }

    /**
     * Keys the input method sends (`InputConnectionAdaptor.handleKeyEvent`): deletion, caret
     * moves, Enter and characters edit the mirror; anything else goes to the view as a
     * hardware key.
     */
    override fun sendKeyEvent(event: KeyEvent): Boolean {
        if (event.action != KeyEvent.ACTION_DOWN) {
            return if (handles(event)) true else super.sendKeyEvent(event)
        }
        val start = Selection.getSelectionStart(mirror).coerceAtLeast(0)
        val end = Selection.getSelectionEnd(mirror).coerceAtLeast(0)
        val (from, to) = minOf(start, end) to maxOf(start, end)
        when (event.keyCode) {
            KeyEvent.KEYCODE_DEL -> {
                if (from != to) mirror.delete(from, to)
                else if (from > 0) mirror.delete(previousBoundary(from), from)
            }
            KeyEvent.KEYCODE_FORWARD_DEL -> {
                if (from != to) mirror.delete(from, to)
                else if (to < mirror.length) mirror.delete(to, nextBoundary(to))
            }
            KeyEvent.KEYCODE_DPAD_LEFT, KeyEvent.KEYCODE_DPAD_RIGHT -> {
                val left = event.keyCode == KeyEvent.KEYCODE_DPAD_LEFT
                val extent = if (left) previousBoundary(end) else nextBoundary(end)
                if (event.isShiftPressed) Selection.setSelection(mirror, start, extent)
                else if (from != to) Selection.setSelection(mirror, if (left) from else to)
                else Selection.setSelection(mirror, extent)
            }
            KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> {
                if (inputType and InputType.TYPE_TEXT_FLAG_MULTI_LINE == 0) {
                    return performEditorAction(imeOptions and EditorInfo.IME_MASK_ACTION)
                }
                mirror.replace(from, to, "\n")
            }
            else -> {
                val character = event.getUnicodeChar(event.metaState)
                if (character == 0) return super.sendKeyEvent(event)
                mirror.replace(from, to, String(Character.toChars(character)))
            }
        }
        return changed(true)
    }

    private fun handles(event: KeyEvent) = when (event.keyCode) {
        KeyEvent.KEYCODE_DEL, KeyEvent.KEYCODE_FORWARD_DEL, KeyEvent.KEYCODE_DPAD_LEFT,
        KeyEvent.KEYCODE_DPAD_RIGHT, KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> true
        else -> event.getUnicodeChar(event.metaState) != 0
    }

    private fun previousBoundary(offset: Int): Int =
        characters().preceding(offset).takeIf { it != BreakIterator.DONE } ?: 0

    private fun nextBoundary(offset: Int): Int =
        characters().following(offset).takeIf { it != BreakIterator.DONE } ?: mirror.length

    private fun characters() = BreakIterator.getCharacterInstance().apply { setText(mirror.toString()) }

    /** Sends the mirror's state to Rust once per change and tells the input method. */
    private fun report() {
        val state = MirrorState(
            mirror.toString(),
            Selection.getSelectionStart(mirror),
            Selection.getSelectionEnd(mirror),
            getComposingSpanStart(mirror),
            getComposingSpanEnd(mirror),
        )
        if (state == view.reported) return
        view.reported = state
        view.reportSelection()
        IncularNative.nativeEditingState(state.text, state.selStart, state.selEnd, state.compStart, state.compEnd)
    }
}

/** A snapshot of the mirror, compared to suppress duplicate reports. */
internal data class MirrorState(val text: String, val selStart: Int, val selEnd: Int, val compStart: Int, val compEnd: Int)
