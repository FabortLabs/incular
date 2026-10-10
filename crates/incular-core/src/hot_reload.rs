//! Hot-patch call points for development builds.
//!
//! A retained closure keeps running the code it was compiled with, so a
//! framework that type-erases an application callback once and calls it for
//! the lifetime of a window would never observe a patch. Calling the callback
//! through [`call`] or [`call_with`] instead resolves its newest body on every
//! invocation. Without the `hot-reload` feature, and in builds without debug
//! assertions, both functions are plain calls.
//!
//! A patch replaces code, never data: the callback's captured values keep the
//! layout they were created with. Changing what a hot callback captures
//! requires a restart.

/// Calls `callback`, using its newest patched body when one is loaded.
#[inline]
pub fn call<F, R>(callback: &mut F) -> R
where
    F: FnMut() -> R,
{
    #[cfg(feature = "hot-reload")]
    {
        // A zero-sized function item keeps the lookup on the monomorphized
        // entry point, which is emitted in the application crate whenever `F`
        // is an application type and is therefore part of every patch.
        fn entry<F, R>(callback: &mut F) -> R
        where
            F: FnMut() -> R,
        {
            callback()
        }
        subsecond::HotFn::current(entry::<F, R>).call((callback,))
    }
    #[cfg(not(feature = "hot-reload"))]
    {
        callback()
    }
}

/// Calls `callback` with `argument`, using its newest patched body when one
/// is loaded.
#[inline]
pub fn call_with<F, A, R>(callback: &mut F, argument: A) -> R
where
    F: FnMut(A) -> R,
{
    #[cfg(feature = "hot-reload")]
    {
        fn entry<F, A, R>(callback: &mut F, argument: A) -> R
        where
            F: FnMut(A) -> R,
        {
            callback(argument)
        }
        subsecond::HotFn::current(entry::<F, A, R>).call((callback, argument))
    }
    #[cfg(not(feature = "hot-reload"))]
    {
        callback(argument)
    }
}

/// Address of the executable's entry symbol in this process. A build driver
/// subtracts it from the address it recorded at link time to undo address
/// space layout randomization.
#[cfg(feature = "hot-reload")]
#[must_use]
pub fn aslr_reference() -> u64 {
    subsecond::aslr_reference() as u64
}

/// A compiled patch produced by the build driver for one running process.
#[cfg(feature = "hot-reload")]
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(transparent)]
pub struct HotPatch(subsecond::JumpTable);

#[cfg(feature = "hot-reload")]
impl HotPatch {
    /// Number of functions this patch redirects.
    #[must_use]
    pub fn function_count(&self) -> usize {
        self.0.map.len()
    }

    /// Loads the patch library and redirects later [`call`] and [`call_with`]
    /// invocations to it. The library stays loaded for the life of the
    /// process, so closures created by earlier code remain callable.
    ///
    /// # Safety
    ///
    /// The patch must have been built against the executable image of this
    /// exact process. A patch for another build redirects calls to unrelated
    /// addresses.
    pub unsafe fn apply(self) -> Result<(), HotPatchError> {
        #[cfg(all(windows, target_arch = "x86_64"))]
        let table = {
            let mut table = self.0;
            table.lib = library_with_safe_trampolines(&table.lib)?;
            table
        };
        #[cfg(not(all(windows, target_arch = "x86_64")))]
        let table = self.0;
        // SAFETY: the caller guarantees the jump table describes this process.
        unsafe { subsecond::apply_patch(table) }.map_err(|error| HotPatchError(error.to_string()))
    }
}

#[cfg(feature = "hot-reload")]
const PE_MACHINE_X86_64: u16 = 0x8664;
#[cfg(feature = "hot-reload")]
const PE_SECTION_EXECUTE: u32 = 0x2000_0000;
#[cfg(feature = "hot-reload")]
const PE_SECTION_HEADER_SIZE: usize = 40;

/// Rewrites the trampolines of a Windows x86-64 patch library so they no
/// longer clobber `RAX`, returning how many were rewritten.
///
/// The build driver reaches functions of the original executable through
/// `mov rax, <address>; jmp rax`. That is harmless for ordinary calls, but
/// the stack probe `__chkstk`, which every function with more than a page of
/// locals calls on entry, takes its size argument in `RAX`: through such a
/// trampoline it probes a garbage amount of stack and the process dies with
/// a stack overflow. `R11` is scratch at every call boundary and is no
/// function's argument, so the trampolines are re-encoded as
/// `mov r11, <address>; jmp r11` in the padding they already own.
///
/// Images for another machine, images that are not PE files and trampolines
/// in any other shape are left untouched.
#[cfg(feature = "hot-reload")]
pub fn retarget_trampolines(image: &mut [u8]) -> usize {
    let mut rewritten = 0;
    for (start, end) in executable_sections(image) {
        let mut offset = start;
        // `48 B8 imm64` `FF E0` `pad`  ->  `49 BB imm64` `41 FF E3`
        while offset + 13 <= end {
            let code = &mut image[offset..offset + 13];
            if code[..2] == [0x48, 0xB8]
                && code[10..12] == [0xFF, 0xE0]
                && matches!(code[12], 0x00 | 0xCC)
            {
                code[..2].copy_from_slice(&[0x49, 0xBB]);
                code[10..].copy_from_slice(&[0x41, 0xFF, 0xE3]);
                rewritten += 1;
                offset += 13;
            } else {
                offset += 1;
            }
        }
    }
    rewritten
}

/// File ranges of the executable sections of an x86-64 PE image.
#[cfg(feature = "hot-reload")]
fn executable_sections(image: &[u8]) -> Vec<(usize, usize)> {
    let u16_at = |offset: usize| {
        let bytes = image.get(offset..offset.checked_add(2)?)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    };
    let u32_at = |offset: usize| {
        let bytes = image.get(offset..offset.checked_add(4)?)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    };
    let sections = || {
        let header = usize::try_from(u32_at(0x3C)?).ok()?;
        if image.get(header..header.checked_add(4)?)? != b"PE\0\0" {
            return None;
        }
        let coff = header + 4;
        if u16_at(coff)? != PE_MACHINE_X86_64 {
            return None;
        }
        let count = usize::from(u16_at(coff + 2)?);
        let table = coff + 20 + usize::from(u16_at(coff + 16)?);
        let mut ranges = Vec::new();
        for index in 0..count {
            let section = table + index * PE_SECTION_HEADER_SIZE;
            let size = usize::try_from(u32_at(section + 16)?).ok()?;
            let start = usize::try_from(u32_at(section + 20)?).ok()?;
            if u32_at(section + 36)? & PE_SECTION_EXECUTE != 0 {
                ranges.push((
                    start.min(image.len()),
                    start.saturating_add(size).min(image.len()),
                ));
            }
        }
        Some(ranges)
    };
    sections().unwrap_or_default()
}

/// Writes a copy of the patch library with
/// [retargeted trampolines](retarget_trampolines) beside it and returns the
/// path to load. A library that needs no rewriting is loaded as it is.
#[cfg(all(feature = "hot-reload", windows, target_arch = "x86_64"))]
fn library_with_safe_trampolines(
    library: &std::path::Path,
) -> Result<std::path::PathBuf, HotPatchError> {
    const SUFFIX: &str = "-incular.dll";
    let error = |error: std::io::Error| HotPatchError(format!("{}: {error}", library.display()));
    let mut image = std::fs::read(library).map_err(error)?;
    if retarget_trampolines(&mut image) == 0 {
        return Ok(library.to_path_buf());
    }
    let stem = library.file_stem().unwrap_or_default().to_string_lossy();
    let repaired = library.with_file_name(format!("{stem}{SUFFIX}"));
    // Copies from earlier runs are no longer mapped; ones this process still
    // has loaded refuse deletion and are skipped.
    if let Some(entries) = library.parent().and_then(|parent| parent.read_dir().ok()) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().ends_with(SUFFIX) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    std::fs::write(&repaired, image).map_err(error)?;
    Ok(repaired)
}

/// The patch library could not be loaded into the process.
#[cfg(feature = "hot-reload")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotPatchError(String);

#[cfg(feature = "hot-reload")]
impl std::fmt::Display for HotPatchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "hot patch was not applied: {}", self.0)
    }
}

#[cfg(feature = "hot-reload")]
impl std::error::Error for HotPatchError {}
