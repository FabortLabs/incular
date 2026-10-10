//! Hot-patch call points are plain calls until a patch is loaded, and a patch
//! that cannot be loaded leaves them that way.

use incular_core::hot_reload::{call, call_with};

#[test]
fn call_points_forward_arguments_results_and_captured_state() {
    let mut total = 0;
    let mut add = |amount: i32| {
        total += amount;
        total
    };
    assert_eq!(call_with(&mut add, 2), 2);
    assert_eq!(call_with(&mut add, 5), 7);

    let mut calls = 0;
    let mut next = || {
        calls += 1;
        calls
    };
    assert_eq!(call(&mut next), 1);
    assert_eq!(call(&mut next), 2);
}

#[test]
fn shared_callbacks_are_callable_through_a_reference() {
    let greet = |name: &str| format!("hello {name}");
    assert_eq!(call_with(&mut &greet, "incular"), "hello incular");
    let answer = || 42;
    assert_eq!(call(&mut &answer), 42);
}

/// The Windows x86-64 build driver reaches the original executable through
/// `mov rax, imm64; jmp rax`, which destroys the size argument of the stack
/// probe. Patch libraries are re-encoded to jump through `r11` before loading.
#[cfg(feature = "hot-reload")]
mod trampolines {
    use incular_core::hot_reload::retarget_trampolines;

    const X86_64: u16 = 0x8664;
    const ARM64: u16 = 0xAA64;
    const EXECUTE: u32 = 0x2000_0020;
    const READ_ONLY_DATA: u32 = 0x4000_0040;
    const TARGET: u64 = 0x7FF6_E9CD_D9D0;

    fn rax_trampoline(padding: u8) -> Vec<u8> {
        let mut code = vec![0x48, 0xB8];
        code.extend_from_slice(&TARGET.to_le_bytes());
        code.extend_from_slice(&[0xFF, 0xE0]);
        code.extend_from_slice(&[padding; 4]);
        code
    }

    fn r11_trampoline(padding: u8) -> Vec<u8> {
        let mut code = vec![0x49, 0xBB];
        code.extend_from_slice(&TARGET.to_le_bytes());
        code.extend_from_slice(&[0x41, 0xFF, 0xE3]);
        code.extend_from_slice(&[padding; 3]);
        code
    }

    /// A minimal PE image: DOS stub, COFF header, section table, raw data.
    fn image(machine: u16, sections: &[(u32, &[u8])]) -> Vec<u8> {
        const HEADER: usize = 0x40;
        let table = HEADER + 4 + 20;
        let mut data_offset = table + sections.len() * 40;
        let mut image = vec![0_u8; data_offset];
        image[0x3C..0x40].copy_from_slice(&(HEADER as u32).to_le_bytes());
        image[HEADER..HEADER + 4].copy_from_slice(b"PE\0\0");
        image[HEADER + 4..HEADER + 6].copy_from_slice(&machine.to_le_bytes());
        image[HEADER + 6..HEADER + 8].copy_from_slice(&(sections.len() as u16).to_le_bytes());
        for (index, (characteristics, data)) in sections.iter().enumerate() {
            let header = table + index * 40;
            image[header + 16..header + 20].copy_from_slice(&(data.len() as u32).to_le_bytes());
            image[header + 20..header + 24].copy_from_slice(&(data_offset as u32).to_le_bytes());
            image[header + 36..header + 40].copy_from_slice(&characteristics.to_le_bytes());
            data_offset += data.len();
        }
        for (_, data) in sections {
            image.extend_from_slice(data);
        }
        image
    }

    #[test]
    fn executable_trampolines_jump_through_r11_and_keep_their_target() {
        let code = [
            vec![0x90; 5],
            rax_trampoline(0x00),
            rax_trampoline(0xCC),
            vec![0xC3],
        ]
        .concat();
        let expected = [
            vec![0x90; 5],
            r11_trampoline(0x00),
            r11_trampoline(0xCC),
            vec![0xC3],
        ]
        .concat();
        let data = rax_trampoline(0x00);
        let mut patched = image(X86_64, &[(EXECUTE, &code), (READ_ONLY_DATA, &data)]);

        assert_eq!(retarget_trampolines(&mut patched), 2);
        assert_eq!(
            patched,
            image(X86_64, &[(EXECUTE, &expected), (READ_ONLY_DATA, &data)]),
            "only executable bytes change, and data that resembles a trampoline is kept"
        );
        assert_eq!(
            retarget_trampolines(&mut patched),
            0,
            "rewriting is idempotent"
        );
    }

    #[test]
    fn code_that_only_resembles_a_trampoline_is_kept() {
        // `mov rax, imm64; jmp rax` running straight into more code has no
        // padding to grow into, so it is not one of the driver's trampolines.
        let mut code = rax_trampoline(0x00);
        code[12] = 0x90;
        let original = image(X86_64, &[(EXECUTE, &code)]);
        let mut patched = original.clone();
        assert_eq!(retarget_trampolines(&mut patched), 0);
        assert_eq!(patched, original);
    }

    #[test]
    fn other_machines_and_malformed_images_are_kept() {
        let code = rax_trampoline(0x00);
        let complete = image(X86_64, &[(EXECUTE, &code)]);
        let mut candidates = vec![
            image(ARM64, &[(EXECUTE, &code)]),
            code.clone(),
            Vec::new(),
            complete[..0x50].to_vec(),
            complete[..complete.len() - code.len()].to_vec(),
        ];
        let mut wrong_signature = complete.clone();
        wrong_signature[0x40] = b'N';
        candidates.push(wrong_signature);
        for original in candidates {
            let mut patched = original.clone();
            assert_eq!(retarget_trampolines(&mut patched), 0);
            assert_eq!(patched, original);
        }
    }
}

#[cfg(feature = "hot-reload")]
mod patches {
    use incular_core::hot_reload::{HotPatch, aslr_reference, call};

    fn patch(library: &str) -> HotPatch {
        serde_json::from_value(serde_json::json!({
            "lib": library,
            "map": { "4096": 8192, "4100": 8200 },
            "aslr_reference": 4096,
            "new_base_address": 8192,
            "ifunc_count": 0,
        }))
        .expect("driver jump table")
    }

    #[test]
    fn patch_reads_the_driver_jump_table() {
        assert_eq!(patch("patch.dll").function_count(), 2);
    }

    #[test]
    fn reference_address_is_stable_within_a_process() {
        assert_eq!(aslr_reference(), aslr_reference());
    }

    #[test]
    fn missing_patch_library_is_reported_and_redirects_nothing() {
        // SAFETY: the library does not exist, so loading fails before any
        // call is redirected.
        let error = unsafe { patch("incular-missing-hot-patch-library").apply() }
            .expect_err("a missing library cannot be applied");
        assert!(error.to_string().starts_with("hot patch was not applied"));
        assert_eq!(call(&mut || 7), 7);
    }
}
