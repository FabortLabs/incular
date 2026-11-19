# Idiomatic Rust audit plan

8 pilot units (U*) at <=1,800 lines, then 334 units (A*) at <=1100 lines; files over the cap are split into line ranges (`path:start-end`). Cap chosen to keep each Haiku 5.5 agent under the 100K-token prompt pricing tier.

Total 286068 source lines.

| Unit | Crate | Lines | Hot | Files |
|---|---|---|---|---|
| U017 | incular-config | 1701 | no | crates/incular-config/src/alignment.rs crates/incular-config/src/constraints.rs crates/incular-config/src/defaults.rs crates/incular-config/src/environment.rs crates/incular-config/src/insets.rs crates/incular-config/src/lib.rs crates/incular-config/src/localization.rs crates/incular-config/src/transient.rs |
| U025 | incular-core | 1680 | no | crates/incular-core/src/arena.rs crates/incular-core/src/context.rs crates/incular-core/src/fast_hash.rs crates/incular-core/src/geometry.rs |
| U026 | incular-core | 1519 | no | crates/incular-core/src/hot_reload.rs crates/incular-core/src/input.rs crates/incular-core/src/key.rs crates/incular-core/src/lib.rs crates/incular-core/src/memory.rs crates/incular-core/src/reactivity.rs crates/incular-core/src/restoration.rs crates/incular-core/src/time.rs crates/incular-core/src/widget.rs |
| U084 | incular-rendering | 169 | yes | crates/incular-rendering/src/backdrop.rs crates/incular-rendering/src/color_space.rs |
| U088 | incular-runtime | 232 | yes | crates/incular-runtime/src/app_log.rs |
| U102 | incular-scroll | 1690 | yes | crates/incular-scroll/src/activity.rs crates/incular-scroll/src/attachment.rs crates/incular-scroll/src/ballistic.rs |
| U106 | incular-text | 1550 | yes | crates/incular-text/src/boundary.rs crates/incular-text/src/editing.rs |
| U123 | incular-widgets | 974 | yes | crates/incular-widgets/examples/mem_probe.rs crates/incular-widgets/src/animated_cross_fade.rs crates/incular-widgets/src/animated_size.rs crates/incular-widgets/src/animated_switcher.rs |
| A001 | incular-accessibility | 581 | no | crates/incular-accessibility/src/lib.rs:1-581 |
| A002 | incular-accessibility | 581 | no | crates/incular-accessibility/src/lib.rs:582-1162 |
| A003 | incular-animation | 920 | yes | crates/incular-animation/src/animations.rs |
| A004 | incular-animation | 655 | yes | crates/incular-animation/src/controller.rs:1-655 |
| A005 | incular-animation | 654 | yes | crates/incular-animation/src/controller.rs:656-1309 |
| A006 | incular-animation | 472 | yes | crates/incular-animation/src/curve2d.rs |
| A007 | incular-animation | 696 | yes | crates/incular-animation/src/curves.rs crates/incular-animation/src/lib.rs |
| A008 | incular-animation | 818 | yes | crates/incular-animation/src/motion.rs |
| A009 | incular-animation | 758 | yes | crates/incular-animation/src/physics.rs |
| A010 | incular-animation | 1053 | yes | crates/incular-animation/src/scheduler.rs crates/incular-animation/src/style.rs crates/incular-animation/src/tween.rs |
| A011 | incular-animation | 341 | yes | crates/incular-animation/src/value.rs |
| A012 | incular-assets | 723 | no | crates/incular-assets/src/bundle.rs crates/incular-assets/src/lib.rs crates/incular-assets/src/network.rs |
| A013 | incular-controls | 850 | no | crates/incular-controls/src/button.rs |
| A014 | incular-controls | 999 | no | crates/incular-controls/src/checkbox.rs crates/incular-controls/src/checkbox_group.rs crates/incular-controls/src/collapsible.rs crates/incular-controls/src/combobox.rs crates/incular-controls/src/composite.rs |
| A015 | incular-controls | 864 | no | crates/incular-controls/src/containers.rs crates/incular-controls/src/context_menu.rs crates/incular-controls/src/dialog.rs crates/incular-controls/src/drawer.rs crates/incular-controls/src/field.rs crates/incular-controls/src/fieldset.rs crates/incular-controls/src/form.rs crates/incular-controls/src/icons.rs |
| A016 | incular-controls | 1073 | no | crates/incular-controls/src/ink.rs crates/incular-controls/src/lib.rs crates/incular-controls/src/menu.rs crates/incular-controls/src/menubar.rs crates/incular-controls/src/meter.rs crates/incular-controls/src/navigation_menu.rs crates/incular-controls/src/number_field.rs crates/incular-controls/src/otp_field.rs |
| A017 | incular-controls | 1052 | no | crates/incular-controls/src/overlay.rs crates/incular-controls/src/popover.rs crates/incular-controls/src/popup.rs crates/incular-controls/src/prelude.rs crates/incular-controls/src/preview_card.rs crates/incular-controls/src/progress.rs crates/incular-controls/src/radio.rs |
| A018 | incular-controls | 902 | no | crates/incular-controls/src/selection.rs crates/incular-controls/src/separator.rs |
| A019 | incular-controls | 934 | no | crates/incular-controls/src/slider.rs |
| A020 | incular-controls | 1024 | no | crates/incular-controls/src/styles.rs |
| A021 | incular-controls | 1032 | no | crates/incular-controls/src/switch.rs crates/incular-controls/src/tabs.rs crates/incular-controls/src/text_context_menu.rs crates/incular-controls/src/text_input.rs |
| A022 | incular-controls | 888 | no | crates/incular-controls/src/theme.rs crates/incular-controls/src/toast.rs |
| A023 | incular-desktop | 932 | no | crates/incular-desktop/src/application_shell.rs crates/incular-desktop/src/browser/clipboard.rs crates/incular-desktop/src/browser/global_shortcuts.rs crates/incular-desktop/src/browser/single_instance.rs crates/incular-desktop/src/clipboard.rs crates/incular-desktop/src/devtools_overlay.rs |
| A024 | incular-desktop | 641 | no | crates/incular-desktop/src/devtools_runner.rs:1-641 |
| A025 | incular-desktop | 1062 | no | crates/incular-desktop/src/devtools_runner.rs:642-1281 crates/incular-desktop/src/display.rs crates/incular-desktop/src/display_identity.rs crates/incular-desktop/src/environment.rs crates/incular-desktop/src/external_drag.rs |
| A026 | incular-desktop | 1070 | no | crates/incular-desktop/src/file_dialogs.rs crates/incular-desktop/src/file_dialogs/linux.rs crates/incular-desktop/src/global_shortcuts.rs crates/incular-desktop/src/host_environment.rs crates/incular-desktop/src/hot_reload.rs crates/incular-desktop/src/input.rs |
| A027 | incular-desktop | 895 | no | crates/incular-desktop/src/lib.rs:1-895 |
| A028 | incular-desktop | 895 | no | crates/incular-desktop/src/lib.rs:896-1790 |
| A029 | incular-desktop | 961 | no | crates/incular-desktop/src/lib.rs:1791-2685 crates/incular-desktop/src/native_requests.rs |
| A030 | incular-desktop | 842 | no | crates/incular-desktop/src/platform_menus.rs crates/incular-desktop/src/platform_services.rs crates/incular-desktop/src/pointer.rs |
| A031 | incular-desktop | 1069 | no | crates/incular-desktop/src/presentation.rs crates/incular-desktop/src/single_instance.rs crates/incular-desktop/src/transient_input.rs |
| A032 | incular-desktop | 900 | no | crates/incular-desktop/src/transients.rs crates/incular-desktop/src/window_host.rs |
| A033 | incular-devtools-protocol | 567 | no | crates/incular-devtools-protocol/src/lib.rs:1-567 |
| A034 | incular-devtools-protocol | 566 | no | crates/incular-devtools-protocol/src/lib.rs:568-1133 |
| A035 | incular-devtools | 730 | no | crates/incular-devtools/src/commands.rs crates/incular-devtools/src/discovery.rs crates/incular-devtools/src/lib.rs |
| A036 | incular-devtools | 678 | no | crates/incular-devtools/src/session.rs tools/incular-devtools/src/activity.rs |
| A037 | incular-devtools | 510 | no | tools/incular-devtools/src/diagnostics.rs |
| A038 | incular-devtools | 897 | no | tools/incular-devtools/src/inspector.rs:1-897 |
| A039 | incular-devtools | 959 | no | tools/incular-devtools/src/inspector.rs:898-1793 tools/incular-devtools/src/lib.rs |
| A040 | incular-devtools | 1043 | no | tools/incular-devtools/src/logo.rs tools/incular-devtools/src/main.rs tools/incular-devtools/src/memory.rs tools/incular-devtools/src/performance.rs tools/incular-devtools/src/property_edit.rs tools/incular-devtools/src/report.rs |
| A041 | incular-devtools | 966 | no | tools/incular-devtools/src/session.rs tools/incular-devtools/src/transport.rs:1-761 |
| A042 | incular-devtools | 760 | no | tools/incular-devtools/src/transport.rs:762-1521 |
| A043 | incular-devtools | 556 | no | tools/incular-devtools/src/views.rs:1-556 |
| A044 | incular-devtools | 813 | no | tools/incular-devtools/src/views.rs:557-1111 tools/incular-devtools/src/views/application.rs tools/incular-devtools/src/views/clipboard.rs |
| A045 | incular-devtools | 825 | no | tools/incular-devtools/src/views/command_palette.rs |
| A046 | incular-devtools | 1025 | no | tools/incular-devtools/src/views/connection.rs tools/incular-devtools/src/views/console.rs tools/incular-devtools/src/views/geometry.rs tools/incular-devtools/src/views/header.rs tools/incular-devtools/src/views/hover.rs |
| A047 | incular-devtools | 656 | no | tools/incular-devtools/src/views/inspection.rs |
| A048 | incular-devtools | 1078 | no | tools/incular-devtools/src/views/memory.rs tools/incular-devtools/src/views/network.rs |
| A049 | incular-devtools | 917 | no | tools/incular-devtools/src/views/overlays.rs tools/incular-devtools/src/views/overview.rs |
| A050 | incular-devtools | 1080 | no | tools/incular-devtools/src/views/performance.rs |
| A051 | incular-devtools | 575 | no | tools/incular-devtools/src/views/property_editor.rs:1-575 |
| A052 | incular-devtools | 736 | no | tools/incular-devtools/src/views/property_editor.rs:576-1149 tools/incular-devtools/src/views/pump.rs |
| A053 | incular-devtools | 556 | no | tools/incular-devtools/src/views/rendering.rs |
| A054 | incular-devtools | 1023 | no | tools/incular-devtools/src/views/shared.rs |
| A055 | incular-devtools | 951 | no | tools/incular-devtools/src/views/shell.rs |
| A056 | incular-devtools | 797 | no | tools/incular-devtools/src/views/theme.rs tools/incular-devtools/src/views/tree.rs |
| A057 | incular-devtools | 796 | no | tools/incular-devtools/src/views/widget_inspector.rs |
| A058 | incular-gestures | 688 | yes | crates/incular-gestures/src/arena.rs crates/incular-gestures/src/arena_member.rs crates/incular-gestures/src/constants.rs |
| A059 | incular-gestures | 620 | yes | crates/incular-gestures/src/details.rs |
| A060 | incular-gestures | 811 | yes | crates/incular-gestures/src/focus.rs:1-811 |
| A061 | incular-gestures | 810 | yes | crates/incular-gestures/src/focus.rs:812-1621 |
| A062 | incular-gestures | 996 | yes | crates/incular-gestures/src/force_press.rs crates/incular-gestures/src/keyboard.rs:1-633 |
| A063 | incular-gestures | 748 | yes | crates/incular-gestures/src/keyboard.rs:634-1265 crates/incular-gestures/src/lib.rs crates/incular-gestures/src/mouse.rs |
| A064 | incular-gestures | 906 | yes | crates/incular-gestures/src/multidrag.rs crates/incular-gestures/src/multitap.rs |
| A065 | incular-gestures | 617 | yes | crates/incular-gestures/src/pointer.rs:1-617 |
| A066 | incular-gestures | 804 | yes | crates/incular-gestures/src/pointer.rs:618-1233 crates/incular-gestures/src/raw.rs |
| A067 | incular-gestures | 663 | yes | crates/incular-gestures/src/scale.rs crates/incular-gestures/src/slop.rs |
| A068 | incular-gestures | 1096 | yes | crates/incular-gestures/src/tap_and_drag.rs crates/incular-gestures/src/velocity.rs |
| A069 | incular-image | 830 | no | crates/incular-image/src/lib.rs |
| A070 | incular-layout | 551 | yes | crates/incular-layout/src/algorithms.rs:1-551 |
| A071 | incular-layout | 551 | yes | crates/incular-layout/src/algorithms.rs:552-1102 |
| A072 | incular-layout | 733 | yes | crates/incular-layout/src/descriptors.rs crates/incular-layout/src/geometry.rs crates/incular-layout/src/lib.rs |
| A073 | incular-linux | 776 | no | crates/incular-linux/src/application_shell.rs crates/incular-linux/src/environment.rs crates/incular-linux/src/lib.rs |
| A074 | incular-macos | 957 | no | crates/incular-macos/src/activation.rs crates/incular-macos/src/application_shell.rs crates/incular-macos/src/environment.rs |
| A075 | incular-macos | 830 | no | crates/incular-macos/src/lib.rs crates/incular-macos/src/platform_menus.rs |
| A076 | incular-material | 1088 | no | crates/incular-material/src/app_shell.rs crates/incular-material/src/app_shell/bars.rs crates/incular-material/src/app_shell/environment.rs |
| A077 | incular-material | 843 | no | crates/incular-material/src/app_shell/material_app.rs crates/incular-material/src/app_shell/navigation.rs |
| A078 | incular-material | 1049 | no | crates/incular-material/src/app_shell/shell.rs crates/incular-material/src/app_shell/vocabulary.rs crates/incular-material/src/buttons.rs crates/incular-material/src/buttons/common.rs |
| A079 | incular-material | 830 | no | crates/incular-material/src/buttons/icon_floating.rs crates/incular-material/src/buttons/style.rs |
| A080 | incular-material | 968 | no | crates/incular-material/src/buttons/variants.rs crates/incular-material/src/buttons/vocabulary.rs crates/incular-material/src/catalog.rs crates/incular-material/src/catalog/about.rs |
| A081 | incular-material | 521 | no | crates/incular-material/src/catalog/animated_icon.rs crates/incular-material/src/catalog/banner.rs crates/incular-material/src/catalog/carousel.rs |
| A082 | incular-material | 875 | no | crates/incular-material/src/catalog/data_table.rs |
| A083 | incular-material | 942 | no | crates/incular-material/src/catalog/expansion.rs crates/incular-material/src/catalog/flexible_space.rs crates/incular-material/src/catalog/grid_tile.rs |
| A084 | incular-material | 758 | no | crates/incular-material/src/catalog/pickers.rs:1-758 |
| A085 | incular-material | 1038 | no | crates/incular-material/src/catalog/pickers.rs:759-1516 crates/incular-material/src/catalog/refresh.rs |
| A086 | incular-material | 1000 | no | crates/incular-material/src/catalog/reorderable.rs crates/incular-material/src/catalog/search.rs crates/incular-material/src/catalog/segmented.rs |
| A087 | incular-material | 597 | no | crates/incular-material/src/catalog/sheets.rs crates/incular-material/src/catalog/stepper.rs crates/incular-material/src/feedback.rs |
| A088 | incular-material | 978 | no | crates/incular-material/src/feedback/dialog.rs crates/incular-material/src/feedback/dialog_handles.rs crates/incular-material/src/feedback/helpers.rs |
| A089 | incular-material | 651 | no | crates/incular-material/src/feedback/indicators.rs crates/incular-material/src/feedback/progress.rs |
| A090 | incular-material | 846 | no | crates/incular-material/src/feedback/transient.rs crates/incular-material/src/feedback/vocabulary.rs crates/incular-material/src/icons.rs crates/incular-material/src/inputs.rs |
| A091 | incular-material | 555 | no | crates/incular-material/src/inputs/input.rs:1-555 |
| A092 | incular-material | 707 | no | crates/incular-material/src/inputs/input.rs:556-1110 crates/incular-material/src/inputs/magnifier.rs |
| A093 | incular-material | 638 | no | crates/incular-material/src/inputs/text_fields.rs:1-638 |
| A094 | incular-material | 1036 | no | crates/incular-material/src/inputs/text_fields.rs:639-1275 crates/incular-material/src/inputs/text_selection_toolbar.rs |
| A095 | incular-material | 841 | no | crates/incular-material/src/lists/chips.rs |
| A096 | incular-material | 985 | no | crates/incular-material/src/lists/list_items.rs crates/incular-material/src/lists/vocabulary.rs crates/incular-material/src/menus.rs |
| A097 | incular-material | 959 | no | crates/incular-material/src/menus/accelerators.rs crates/incular-material/src/menus/anchor.rs |
| A098 | incular-material | 725 | no | crates/incular-material/src/menus/controls.rs |
| A099 | incular-material | 1014 | no | crates/incular-material/src/menus/dropdown.rs |
| A100 | incular-material | 897 | no | crates/incular-material/src/menus/popup.rs |
| A101 | incular-material | 886 | no | crates/incular-material/src/menus/style.rs crates/incular-material/src/menus/vocabulary.rs crates/incular-material/src/navigation.rs crates/incular-material/src/navigation/bars.rs |
| A102 | incular-material | 667 | no | crates/incular-material/src/navigation/tabs.rs crates/incular-material/src/navigation/vocabulary.rs |
| A103 | incular-material | 1009 | no | crates/incular-material/src/page_transitions.rs crates/incular-material/src/selection.rs crates/incular-material/src/selection/checkbox_radio.rs |
| A104 | incular-material | 1025 | no | crates/incular-material/src/selection/slider_range.rs |
| A105 | incular-material | 880 | no | crates/incular-material/src/selection/switch.rs crates/incular-material/src/selection/vocabulary.rs crates/incular-material/src/surfaces.rs crates/incular-material/src/surfaces/components.rs crates/incular-material/src/surfaces/ink.rs |
| A106 | incular-material | 1051 | no | crates/incular-material/src/surfaces/material.rs crates/incular-material/src/surfaces/vocabulary.rs crates/incular-material/src/theme.rs crates/incular-material/src/theme/components.rs crates/incular-material/src/theme/helpers.rs |
| A107 | incular-material | 1096 | no | crates/incular-material/src/theme/theme_data.rs |
| A108 | incular-navigation | 884 | no | crates/incular-navigation/src/hero_controller.rs crates/incular-navigation/src/lib.rs crates/incular-navigation/src/navigator.rs:1-690 |
| A109 | incular-navigation | 850 | no | crates/incular-navigation/src/navigator.rs:691-1380 crates/incular-navigation/src/predictive_back.rs |
| A110 | incular-navigation | 1072 | no | crates/incular-navigation/src/presentation.rs crates/incular-navigation/src/registry.rs |
| A111 | incular-navigation | 771 | no | crates/incular-navigation/src/route_data.rs crates/incular-navigation/src/route_observer.rs |
| A112 | incular-platform | 1088 | no | crates/incular-platform/src/application_activation.rs crates/incular-platform/src/application_shell.rs crates/incular-platform/src/back_gesture.rs crates/incular-platform/src/capabilities.rs crates/incular-platform/src/content_sensitivity.rs |
| A113 | incular-platform | 1041 | no | crates/incular-platform/src/data_transfer.rs crates/incular-platform/src/display.rs |
| A114 | incular-platform | 634 | no | crates/incular-platform/src/file_dialog.rs |
| A115 | incular-platform | 1056 | no | crates/incular-platform/src/lib.rs crates/incular-platform/src/operation.rs crates/incular-platform/src/platform_view.rs crates/incular-platform/src/pointer.rs |
| A116 | incular-platform | 557 | no | crates/incular-platform/src/spell_check.rs crates/incular-platform/src/system_environment.rs crates/incular-platform/src/undo_manager.rs crates/incular-platform/src/window_control.rs |
| A117 | incular-rendering | 929 | yes | crates/incular-rendering/src/compositor.rs:1-929 |
| A118 | incular-rendering | 929 | yes | crates/incular-rendering/src/compositor.rs:930-1858 |
| A119 | incular-rendering | 927 | yes | crates/incular-rendering/src/compositor.rs:1859-2785 |
| A120 | incular-rendering | 680 | yes | crates/incular-rendering/src/display_list.rs |
| A121 | incular-rendering | 552 | yes | crates/incular-rendering/src/effects.rs:1-552 |
| A122 | incular-rendering | 740 | yes | crates/incular-rendering/src/effects.rs:553-1103 crates/incular-rendering/src/geometry.rs crates/incular-rendering/src/glyphs.rs |
| A123 | incular-rendering | 680 | yes | crates/incular-rendering/src/gradients.rs crates/incular-rendering/src/lib.rs crates/incular-rendering/src/paint.rs |
| A124 | incular-rendering | 870 | yes | crates/incular-rendering/src/paths.rs crates/incular-rendering/src/rsuperellipse.rs |
| A125 | incular-runtime | 943 | yes | crates/incular-runtime/src/application.rs:1-943 |
| A126 | incular-runtime | 943 | yes | crates/incular-runtime/src/application.rs:944-1886 |
| A127 | incular-runtime | 942 | yes | crates/incular-runtime/src/application.rs:1887-2828 |
| A128 | incular-runtime | 235 | yes | crates/incular-runtime/src/application_activations.rs |
| A129 | incular-runtime | 988 | yes | crates/incular-runtime/src/application_shell.rs |
| A130 | incular-runtime | 639 | yes | crates/incular-runtime/src/application_types.rs crates/incular-runtime/src/context.rs crates/incular-runtime/src/devtools_inspection.rs |
| A131 | incular-runtime | 993 | yes | crates/incular-runtime/src/environment.rs |
| A132 | incular-runtime | 754 | yes | crates/incular-runtime/src/executor.rs crates/incular-runtime/src/file_dialogs.rs |
| A133 | incular-runtime | 945 | yes | crates/incular-runtime/src/frame.rs:1-945 |
| A134 | incular-runtime | 945 | yes | crates/incular-runtime/src/frame.rs:946-1890 |
| A135 | incular-runtime | 945 | yes | crates/incular-runtime/src/frame.rs:1891-2835 |
| A136 | incular-runtime | 594 | yes | crates/incular-runtime/src/frame/text_actions.rs |
| A137 | incular-runtime | 956 | yes | crates/incular-runtime/src/frame/text_gestures.rs |
| A138 | incular-runtime | 602 | yes | crates/incular-runtime/src/global_shortcuts.rs crates/incular-runtime/src/lib.rs |
| A139 | incular-runtime | 635 | yes | crates/incular-runtime/src/profiling.rs |
| A140 | incular-runtime | 965 | yes | crates/incular-runtime/src/reactive.rs crates/incular-runtime/src/request_admission.rs |
| A141 | incular-runtime | 860 | yes | crates/incular-runtime/src/request_channel.rs crates/incular-runtime/src/request_registry.rs crates/incular-runtime/src/restoration.rs:1-612 |
| A142 | incular-runtime | 812 | yes | crates/incular-runtime/src/restoration.rs:613-1223 crates/incular-runtime/src/route_focus.rs |
| A143 | incular-runtime | 858 | yes | crates/incular-runtime/src/route_outlet.rs:1-858 |
| A144 | incular-runtime | 924 | yes | crates/incular-runtime/src/route_outlet.rs:859-1716 crates/incular-runtime/src/route_tasks.rs |
| A145 | incular-runtime | 693 | yes | crates/incular-runtime/src/simulation.rs:1-693 |
| A146 | incular-runtime | 693 | yes | crates/incular-runtime/src/simulation.rs:694-1386 |
| A147 | incular-runtime | 989 | yes | crates/incular-runtime/src/tasks.rs |
| A148 | incular-runtime | 908 | yes | crates/incular-runtime/src/text_shortcuts.rs crates/incular-runtime/src/transient_presentation.rs crates/incular-runtime/src/undo.rs |
| A149 | incular-runtime | 542 | yes | crates/incular-runtime/src/window_commands.rs |
| A150 | incular-runtime | 783 | yes | crates/incular-runtime/src/window_state.rs |
| A151 | incular-scroll | 608 | yes | crates/incular-scroll/src/controller.rs |
| A152 | incular-scroll | 628 | yes | crates/incular-scroll/src/coordinator.rs |
| A153 | incular-scroll | 997 | yes | crates/incular-scroll/src/extent_index.rs crates/incular-scroll/src/lib.rs crates/incular-scroll/src/metrics.rs crates/incular-scroll/src/notifications.rs |
| A154 | incular-scroll | 555 | yes | crates/incular-scroll/src/physics.rs:1-555 |
| A155 | incular-scroll | 975 | yes | crates/incular-scroll/src/physics.rs:556-1110 crates/incular-scroll/src/restoration.rs crates/incular-scroll/src/scrollbar.rs crates/incular-scroll/src/sliver.rs |
| A156 | incular-semantics | 622 | no | crates/incular-semantics/src/lib.rs |
| A157 | incular-text | 810 | yes | crates/incular-text/src/engine.rs:1-810 |
| A158 | incular-text | 810 | yes | crates/incular-text/src/engine.rs:811-1620 |
| A159 | incular-text | 808 | yes | crates/incular-text/src/engine.rs:1621-2428 |
| A160 | incular-text | 869 | yes | crates/incular-text/src/engine/document.rs crates/incular-text/src/engine/inline.rs crates/incular-text/src/engine/pool.rs |
| A161 | incular-text | 1083 | yes | crates/incular-text/src/geometry.rs crates/incular-text/src/lib.rs crates/incular-text/src/spans.rs crates/incular-text/src/style.rs crates/incular-text/src/style/fonts.rs |
| A162 | incular-text | 407 | yes | crates/incular-text/src/style/icons.rs crates/incular-text/src/style/text.rs |
| A163 | incular-text | 761 | yes | crates/incular-text/src/style/text_style.rs |
| A164 | incular-web | 887 | no | crates/incular-web/src/clipboard.rs crates/incular-web/src/environment.rs crates/incular-web/src/fonts.rs |
| A165 | incular-web | 797 | no | crates/incular-web/src/history.rs crates/incular-web/src/lib.rs crates/incular-web/src/page_events.rs crates/incular-web/src/segmentation.rs |
| A166 | incular-web | 654 | no | crates/incular-web/src/semantics.rs |
| A167 | incular-web | 540 | no | crates/incular-web/src/text_input.rs |
| A168 | incular-wgpu | 704 | yes | crates/incular-wgpu/build.rs crates/incular-wgpu/src/batching.rs crates/incular-wgpu/src/built_in_shaders.rs crates/incular-wgpu/src/compositor.rs crates/incular-wgpu/src/constants.rs crates/incular-wgpu/src/deferred_pipeline.rs |
| A169 | incular-wgpu | 1089 | yes | crates/incular-wgpu/src/diagnostics.rs crates/incular-wgpu/src/driver_workarounds.rs crates/incular-wgpu/src/fast_hash.rs crates/incular-wgpu/src/geometry.rs crates/incular-wgpu/src/glyph_position.rs |
| A170 | incular-wgpu | 433 | yes | crates/incular-wgpu/src/glyph_rasterizer.rs crates/incular-wgpu/src/glyph_uploads.rs |
| A171 | incular-wgpu | 1074 | yes | crates/incular-wgpu/src/glyphs.rs |
| A172 | incular-wgpu | 898 | yes | crates/incular-wgpu/src/lib.rs crates/incular-wgpu/src/memory_report.rs crates/incular-wgpu/src/pipelines.rs:1-664 |
| A173 | incular-wgpu | 974 | yes | crates/incular-wgpu/src/pipelines.rs:665-1327 crates/incular-wgpu/src/render_passes.rs |
| A174 | incular-wgpu | 586 | yes | crates/incular-wgpu/src/render_plan.rs:1-586 |
| A175 | incular-wgpu | 1075 | yes | crates/incular-wgpu/src/render_plan.rs:587-1171 crates/incular-wgpu/src/renderer.rs crates/incular-wgpu/src/renderer/backdrop.rs |
| A176 | incular-wgpu | 659 | yes | crates/incular-wgpu/src/renderer/backdrop_cache.rs crates/incular-wgpu/src/renderer/capture.rs |
| A177 | incular-wgpu | 562 | yes | crates/incular-wgpu/src/renderer/effects.rs:1-562 |
| A178 | incular-wgpu | 562 | yes | crates/incular-wgpu/src/renderer/effects.rs:563-1124 |
| A179 | incular-wgpu | 919 | yes | crates/incular-wgpu/src/renderer/frame.rs |
| A180 | incular-wgpu | 573 | yes | crates/incular-wgpu/src/renderer/lowering.rs:1-573 |
| A181 | incular-wgpu | 1033 | yes | crates/incular-wgpu/src/renderer/lowering.rs:574-1145 crates/incular-wgpu/src/renderer/matrix_filter.rs crates/incular-wgpu/src/renderer/memory.rs |
| A182 | incular-wgpu | 701 | yes | crates/incular-wgpu/src/renderer/rendering.rs |
| A183 | incular-wgpu | 511 | yes | crates/incular-wgpu/src/renderer/resources.rs |
| A184 | incular-wgpu | 656 | yes | crates/incular-wgpu/src/renderer/state.rs |
| A185 | incular-wgpu | 933 | yes | crates/incular-wgpu/src/resources.rs:1-933 |
| A186 | incular-wgpu | 932 | yes | crates/incular-wgpu/src/resources.rs:934-1865 |
| A187 | incular-wgpu | 181 | yes | crates/incular-wgpu/src/surface.rs |
| A188 | incular-widgets | 632 | yes | crates/incular-widgets/src/advanced_scrolling/draggable.rs:1-632 |
| A189 | incular-widgets | 666 | yes | crates/incular-widgets/src/advanced_scrolling/draggable.rs:633-1264 crates/incular-widgets/src/advanced_scrolling/mod.rs |
| A190 | incular-widgets | 788 | yes | crates/incular-widgets/src/advanced_scrolling/raw_scrollbar.rs |
| A191 | incular-widgets | 607 | yes | crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:1-607 |
| A192 | incular-widgets | 606 | yes | crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:608-1213 |
| A193 | incular-widgets | 1085 | yes | crates/incular-widgets/src/advanced_scrolling/wheel.rs |
| A194 | incular-widgets | 653 | yes | crates/incular-widgets/src/advanced_slivers/animated.rs:1-653 |
| A195 | incular-widgets | 1040 | yes | crates/incular-widgets/src/advanced_slivers/animated.rs:654-1305 crates/incular-widgets/src/advanced_slivers/keep_alive.rs crates/incular-widgets/src/advanced_slivers/mod.rs |
| A196 | incular-widgets | 941 | yes | crates/incular-widgets/src/advanced_slivers/tree_sliver.rs |
| A197 | incular-widgets | 882 | yes | crates/incular-widgets/src/animation_transitions.rs |
| A198 | incular-widgets | 1068 | yes | crates/incular-widgets/src/app_shell/app.rs crates/incular-widgets/src/app_shell/mod.rs |
| A199 | incular-widgets | 926 | yes | crates/incular-widgets/src/app_shell/router.rs:1-926 |
| A200 | incular-widgets | 925 | yes | crates/incular-widgets/src/app_shell/router.rs:927-1851 |
| A201 | incular-widgets | 341 | yes | crates/incular-widgets/src/app_shell/title.rs |
| A202 | incular-widgets | 1082 | yes | crates/incular-widgets/src/app_shell/view.rs crates/incular-widgets/src/app_shell/window_chrome.rs |
| A203 | incular-widgets | 877 | yes | crates/incular-widgets/src/asset_bundle.rs crates/incular-widgets/src/async_builders.rs crates/incular-widgets/src/back_gesture.rs |
| A204 | incular-widgets | 828 | yes | crates/incular-widgets/src/compositing.rs crates/incular-widgets/src/context_menu.rs |
| A205 | incular-widgets | 703 | yes | crates/incular-widgets/src/custom_layout.rs |
| A206 | incular-widgets | 975 | yes | crates/incular-widgets/src/devtools.rs |
| A207 | incular-widgets | 849 | yes | crates/incular-widgets/src/devtools_edit.rs crates/incular-widgets/src/devtools_props.rs |
| A208 | incular-widgets | 1044 | yes | crates/incular-widgets/src/devtools_trees.rs crates/incular-widgets/src/dismissible.rs |
| A209 | incular-widgets | 369 | yes | crates/incular-widgets/src/display_feature_sub_screen.rs crates/incular-widgets/src/drag_boundary.rs |
| A210 | incular-widgets | 794 | yes | crates/incular-widgets/src/drag_drop.rs crates/incular-widgets/src/dx.rs |
| A211 | incular-widgets | 1080 | yes | crates/incular-widgets/src/dx/animate.rs crates/incular-widgets/src/dx/drag.rs crates/incular-widgets/src/dx/motion/behavior.rs crates/incular-widgets/src/dx/motion/cond.rs |
| A212 | incular-widgets | 1055 | yes | crates/incular-widgets/src/dx/motion/config.rs crates/incular-widgets/src/dx/motion/dnd.rs crates/incular-widgets/src/dx/motion/dragging.rs crates/incular-widgets/src/dx/motion/geometry.rs |
| A213 | incular-widgets | 827 | yes | crates/incular-widgets/src/dx/motion/layout.rs crates/incular-widgets/src/dx/motion/lint.rs crates/incular-widgets/src/dx/motion/mod.rs |
| A214 | incular-widgets | 601 | yes | crates/incular-widgets/src/dx/motion/node.rs:1-601 |
| A215 | incular-widgets | 986 | yes | crates/incular-widgets/src/dx/motion/node.rs:602-1201 crates/incular-widgets/src/dx/motion/pose.rs |
| A216 | incular-widgets | 876 | yes | crates/incular-widgets/src/dx/motion/presence.rs crates/incular-widgets/src/dx/motion/reorder.rs:1-671 |
| A217 | incular-widgets | 1033 | yes | crates/incular-widgets/src/dx/motion/reorder.rs:672-1342 crates/incular-widgets/src/dx/motion/shared.rs |
| A218 | incular-widgets | 990 | yes | crates/incular-widgets/src/dx/motion/timeline.rs |
| A219 | incular-widgets | 1073 | yes | crates/incular-widgets/src/dx/responsive.rs crates/incular-widgets/src/dx/styled.rs crates/incular-widgets/src/dx/transition.rs crates/incular-widgets/src/dx/view_transition.rs |
| A220 | incular-widgets | 768 | yes | crates/incular-widgets/src/environment.rs |
| A221 | incular-widgets | 1095 | yes | crates/incular-widgets/src/explicit_animations.rs crates/incular-widgets/src/extensions.rs crates/incular-widgets/src/external_drop.rs crates/incular-widgets/src/fade_in_image.rs |
| A222 | incular-widgets | 413 | yes | crates/incular-widgets/src/fast_hash.rs crates/incular-widgets/src/flutter_logo.rs |
| A223 | incular-widgets | 842 | yes | crates/incular-widgets/src/focus_keyboard.rs |
| A224 | incular-widgets | 738 | yes | crates/incular-widgets/src/forms.rs:1-738 |
| A225 | incular-widgets | 738 | yes | crates/incular-widgets/src/forms.rs:739-1476 |
| A226 | incular-widgets | 643 | yes | crates/incular-widgets/src/gestures.rs |
| A227 | incular-widgets | 861 | yes | crates/incular-widgets/src/hero.rs |
| A228 | incular-widgets | 917 | yes | crates/incular-widgets/src/implicit_animations.rs:1-917 |
| A229 | incular-widgets | 956 | yes | crates/incular-widgets/src/implicit_animations.rs:918-1833 crates/incular-widgets/src/indexed_semantics.rs |
| A230 | incular-widgets | 373 | yes | crates/incular-widgets/src/inherited_data.rs |
| A231 | incular-widgets | 878 | yes | crates/incular-widgets/src/interactive_viewer.rs |
| A232 | incular-widgets | 977 | yes | crates/incular-widgets/src/internal.rs crates/incular-widgets/src/layout/basic.rs crates/incular-widgets/src/layout/basic/baseline_aspect_fractional.rs crates/incular-widgets/src/layout/basic/clipping.rs |
| A233 | incular-widgets | 520 | yes | crates/incular-widgets/src/layout/basic/padding_alignment.rs crates/incular-widgets/src/layout/basic/painting.rs |
| A234 | incular-widgets | 1032 | yes | crates/incular-widgets/src/layout/basic/sizing_constraints.rs crates/incular-widgets/src/layout/basic/split_view.rs crates/incular-widgets/src/layout/basic/transforms.rs |
| A235 | incular-widgets | 731 | yes | crates/incular-widgets/src/layout/basic/utility.rs crates/incular-widgets/src/layout/basic/visibility.rs crates/incular-widgets/src/layout/container.rs |
| A236 | incular-widgets | 886 | yes | crates/incular-widgets/src/layout/flex.rs crates/incular-widgets/src/layout/mod.rs crates/incular-widgets/src/layout/stack.rs crates/incular-widgets/src/layout/table.rs crates/incular-widgets/src/layout/wrap.rs |
| A237 | incular-widgets | 716 | yes | crates/incular-widgets/src/lib.rs crates/incular-widgets/src/macros.rs crates/incular-widgets/src/magnifier.rs |
| A238 | incular-widgets | 971 | yes | crates/incular-widgets/src/navigation/back_dispatch.rs crates/incular-widgets/src/navigation/barrier.rs crates/incular-widgets/src/navigation/mod.rs |
| A239 | incular-widgets | 986 | yes | crates/incular-widgets/src/navigation/page_storage.rs crates/incular-widgets/src/navigation/pop_scopes.rs crates/incular-widgets/src/navigation/restoration.rs |
| A240 | incular-widgets | 358 | yes | crates/incular-widgets/src/navigation_scopes.rs crates/incular-widgets/src/notification.rs |
| A241 | incular-widgets | 951 | yes | crates/incular-widgets/src/overscroll_indicator.rs crates/incular-widgets/src/painting_effects.rs |
| A242 | incular-widgets | 965 | yes | crates/incular-widgets/src/painting_effects/box_painter.rs |
| A243 | incular-widgets | 1064 | yes | crates/incular-widgets/src/painting_effects/custom_paint.rs crates/incular-widgets/src/painting_effects/decoration.rs crates/incular-widgets/src/painting_effects/geometry.rs |
| A244 | incular-widgets | 782 | yes | crates/incular-widgets/src/painting_effects/images.rs crates/incular-widgets/src/painting_effects/layers.rs |
| A245 | incular-widgets | 608 | yes | crates/incular-widgets/src/painting_effects/shape_borders.rs:1-608 |
| A246 | incular-widgets | 1090 | yes | crates/incular-widgets/src/painting_effects/shape_borders.rs:609-1215 crates/incular-widgets/src/painting_effects/shape_decoration.rs crates/incular-widgets/src/platform_view.rs |
| A247 | incular-widgets | 590 | yes | crates/incular-widgets/src/platform_widgets/menu.rs:1-590 |
| A248 | incular-widgets | 1070 | yes | crates/incular-widgets/src/platform_widgets/menu.rs:591-1179 crates/incular-widgets/src/platform_widgets/mod.rs crates/incular-widgets/src/probe.rs crates/incular-widgets/src/provider_image.rs crates/incular-widgets/src/radio_selection.rs |
| A249 | incular-widgets | 1046 | yes | crates/incular-widgets/src/raw_input.rs |
| A250 | incular-widgets | 578 | yes | crates/incular-widgets/src/raw_menu_anchor.rs |
| A251 | incular-widgets | 712 | yes | crates/incular-widgets/src/raw_tooltip.rs:1-712 |
| A252 | incular-widgets | 712 | yes | crates/incular-widgets/src/raw_tooltip.rs:713-1424 |
| A253 | incular-widgets | 459 | yes | crates/incular-widgets/src/recursion.rs |
| A254 | incular-widgets | 963 | yes | crates/incular-widgets/src/render_object/layers.rs |
| A255 | incular-widgets | 956 | yes | crates/incular-widgets/src/render_object/mod.rs crates/incular-widgets/src/render_object/update.rs |
| A256 | incular-widgets | 986 | yes | crates/incular-widgets/src/repeating_animation_builder.rs crates/incular-widgets/src/safe_area.rs crates/incular-widgets/src/scroll_aware_image_provider.rs crates/incular-widgets/src/scroll_behavior.rs |
| A257 | incular-widgets | 917 | yes | crates/incular-widgets/src/scrolling.rs crates/incular-widgets/src/scrolling/child_delegates.rs crates/incular-widgets/src/scrolling/ensure_visible.rs crates/incular-widgets/src/scrolling/geometry.rs |
| A258 | incular-widgets | 852 | yes | crates/incular-widgets/src/scrolling/layout.rs:1-852 |
| A259 | incular-widgets | 852 | yes | crates/incular-widgets/src/scrolling/layout.rs:853-1704 |
| A260 | incular-widgets | 852 | yes | crates/incular-widgets/src/scrolling/layout.rs:1705-2556 |
| A261 | incular-widgets | 936 | yes | crates/incular-widgets/src/scrolling/layout.rs:2557-3408 crates/incular-widgets/src/scrolling/notifications.rs |
| A262 | incular-widgets | 571 | yes | crates/incular-widgets/src/scrolling/reorderable.rs |
| A263 | incular-widgets | 798 | yes | crates/incular-widgets/src/scrolling/scroll_views.rs:1-798 |
| A264 | incular-widgets | 798 | yes | crates/incular-widgets/src/scrolling/scroll_views.rs:799-1596 |
| A265 | incular-widgets | 895 | yes | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1-895 |
| A266 | incular-widgets | 895 | yes | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:896-1790 |
| A267 | incular-widgets | 894 | yes | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1791-2684 |
| A268 | incular-widgets | 1041 | yes | crates/incular-widgets/src/scrolling/sliver_fade.rs crates/incular-widgets/src/scrolling/sliver_lists.rs:1-737 |
| A269 | incular-widgets | 933 | yes | crates/incular-widgets/src/scrolling/sliver_lists.rs:738-1473 crates/incular-widgets/src/scrolling/sliver_semantics.rs |
| A270 | incular-widgets | 713 | yes | crates/incular-widgets/src/selection.rs crates/incular-widgets/src/selection_container.rs crates/incular-widgets/src/selection_listener.rs |
| A271 | incular-widgets | 1083 | yes | crates/incular-widgets/src/semantics.rs crates/incular-widgets/src/semantics_debugger.rs crates/incular-widgets/src/shortcut_registry.rs crates/incular-widgets/src/size_changed.rs |
| A272 | incular-widgets | 1044 | yes | crates/incular-widgets/src/spell_check.rs crates/incular-widgets/src/transient.rs |
| A273 | incular-widgets | 857 | yes | crates/incular-widgets/src/tree.rs:1-857 |
| A274 | incular-widgets | 856 | yes | crates/incular-widgets/src/tree.rs:858-1713 |
| A275 | incular-widgets | 803 | yes | crates/incular-widgets/src/tree/context.rs crates/incular-widgets/src/tree/departing.rs |
| A276 | incular-widgets | 752 | yes | crates/incular-widgets/src/tree/descriptors.rs:1-752 |
| A277 | incular-widgets | 752 | yes | crates/incular-widgets/src/tree/descriptors.rs:753-1504 |
| A278 | incular-widgets | 910 | yes | crates/incular-widgets/src/tree/descriptors.rs:1505-2254 crates/incular-widgets/src/tree/external_drop.rs |
| A279 | incular-widgets | 941 | yes | crates/incular-widgets/src/tree/focus.rs crates/incular-widgets/src/tree/global_keys.rs |
| A280 | incular-widgets | 974 | yes | crates/incular-widgets/src/tree/interaction.rs:1-974 |
| A281 | incular-widgets | 974 | yes | crates/incular-widgets/src/tree/interaction.rs:975-1948 |
| A282 | incular-widgets | 974 | yes | crates/incular-widgets/src/tree/interaction.rs:1949-2922 |
| A283 | incular-widgets | 972 | yes | crates/incular-widgets/src/tree/interaction.rs:2923-3894 |
| A284 | incular-widgets | 842 | yes | crates/incular-widgets/src/tree/invariants.rs |
| A285 | incular-widgets | 580 | yes | crates/incular-widgets/src/tree/layout.rs:1-580 |
| A286 | incular-widgets | 580 | yes | crates/incular-widgets/src/tree/layout.rs:581-1160 |
| A287 | incular-widgets | 745 | yes | crates/incular-widgets/src/tree/layout/behavior/containers.rs |
| A288 | incular-widgets | 827 | yes | crates/incular-widgets/src/tree/layout/behavior/core.rs crates/incular-widgets/src/tree/layout/behavior/effects.rs crates/incular-widgets/src/tree/layout/behavior/mod.rs crates/incular-widgets/src/tree/layout/behavior/scrolling.rs |
| A289 | incular-widgets | 842 | yes | crates/incular-widgets/src/tree/layout/behavior/sliver.rs crates/incular-widgets/src/tree/layout/behavior/text.rs |
| A290 | incular-widgets | 691 | yes | crates/incular-widgets/src/tree/painting.rs:1-691 |
| A291 | incular-widgets | 690 | yes | crates/incular-widgets/src/tree/painting.rs:692-1381 |
| A292 | incular-widgets | 938 | yes | crates/incular-widgets/src/tree/painting/behavior/content.rs crates/incular-widgets/src/tree/painting/behavior/mod.rs crates/incular-widgets/src/tree/painting/behavior/scrolling.rs crates/incular-widgets/src/tree/painting/behavior/shader_mask.rs crates/incular-widgets/src/tree/painting/behavior/visual.rs |
| A293 | incular-widgets | 567 | yes | crates/incular-widgets/src/tree/raw_input.rs:1-567 |
| A294 | incular-widgets | 566 | yes | crates/incular-widgets/src/tree/raw_input.rs:568-1133 |
| A295 | incular-widgets | 833 | yes | crates/incular-widgets/src/tree/reconciliation.rs:1-833 |
| A296 | incular-widgets | 833 | yes | crates/incular-widgets/src/tree/reconciliation.rs:834-1666 |
| A297 | incular-widgets | 832 | yes | crates/incular-widgets/src/tree/reconciliation.rs:1667-2498 |
| A298 | incular-widgets | 904 | yes | crates/incular-widgets/src/tree/rendering.rs crates/incular-widgets/src/tree/rendering/lowering/effects.rs crates/incular-widgets/src/tree/rendering/lowering/layout.rs crates/incular-widgets/src/tree/rendering/lowering/mod.rs crates/incular-widgets/src/tree/rendering/lowering/scrolling.rs |
| A299 | incular-widgets | 909 | yes | crates/incular-widgets/src/tree/rendering/lowering/visual.rs crates/incular-widgets/src/tree/retained.rs:1-699 |
| A300 | incular-widgets | 770 | yes | crates/incular-widgets/src/tree/retained.rs:700-1397 crates/incular-widgets/src/tree/retained_state.rs |
| A301 | incular-widgets | 699 | yes | crates/incular-widgets/src/tree/semantics.rs:1-699 |
| A302 | incular-widgets | 698 | yes | crates/incular-widgets/src/tree/semantics.rs:700-1397 |
| A303 | incular-widgets | 868 | yes | crates/incular-widgets/src/tree/specs.rs |
| A304 | incular-widgets | 972 | yes | crates/incular-widgets/src/tree/text.rs:1-972 |
| A305 | incular-widgets | 972 | yes | crates/incular-widgets/src/tree/text.rs:973-1944 |
| A306 | incular-widgets | 1013 | yes | crates/incular-widgets/src/tree/values.rs |
| A307 | incular-widgets | 837 | yes | crates/incular-widgets/src/tree/widget/constructors.rs:1-837 |
| A308 | incular-widgets | 1024 | yes | crates/incular-widgets/src/tree/widget/constructors.rs:838-1674 crates/incular-widgets/src/tree/widget/geometry.rs |
| A309 | incular-widgets | 979 | yes | crates/incular-widgets/src/tree/widget/mod.rs crates/incular-widgets/src/tree/widget/structure.rs:1-845 |
| A310 | incular-widgets | 845 | yes | crates/incular-widgets/src/tree/widget/structure.rs:846-1690 |
| A311 | incular-widgets | 892 | yes | crates/incular-widgets/src/tree/widget/structure.rs:1691-2534 crates/incular-widgets/src/tweens.rs crates/incular-widgets/src/util.rs |
| A312 | incular-widgets | 429 | yes | crates/incular-widgets/src/utilities.rs |
| A313 | incular-windows | 1050 | no | crates/incular-windows/src/application_shell.rs crates/incular-windows/src/crash_reporter.rs crates/incular-windows/src/environment.rs crates/incular-windows/src/lib.rs |
| A314 | incular-windows | 949 | no | crates/incular-windows/src/platform_menus.rs crates/incular-windows/src/pointer.rs |
| A315 | incular | 896 | no | harness/incular/src/lib.rs |
| A316 | examples | 794 | no | examples/animation/main.rs examples/async_runtime/main.rs examples/base_ui_controls/main.rs examples/canvas_layers/main.rs examples/color_effects/main.rs |
| A317 | examples | 929 | no | examples/controls_gallery/main.rs examples/core_defaults/main.rs examples/counter/main.rs examples/custom/main.rs examples/devtools_playground/console_lab.rs examples/devtools_playground/editable.rs |
| A318 | examples | 1089 | no | examples/devtools_playground/jank.rs examples/devtools_playground/layout_lab.rs examples/devtools_playground/main.rs examples/devtools_playground/motion.rs examples/devtools_playground/semantics_lab.rs examples/devtools_playground/signals_lab.rs examples/devtools_playground/text_lab.rs |
| A319 | examples | 958 | no | examples/devtools_playground/theme.rs examples/effects/main.rs examples/environment/main.rs examples/flutter_widgets_gallery/main.rs examples/gesture_gallery/main.rs examples/hot_reload/main.rs examples/images/main.rs examples/interaction_complete/main.rs |
| A320 | examples | 1050 | no | examples/issue_tracker/main.rs examples/layout_complete/main.rs examples/layout_gallery/main.rs |
| A321 | examples | 1007 | no | examples/localization/main.rs examples/material_gallery/main.rs examples/material_workbench/main.rs examples/motion/main.rs |
| A322 | examples | 592 | no | examples/multi_window/main.rs examples/navigation_showcase/main.rs examples/opacity/main.rs examples/painting/main.rs |
| A323 | examples | 1016 | no | examples/performance_gallery/main.rs examples/reactive/main.rs |
| A324 | examples | 1030 | no | examples/responsive_layout/main.rs examples/restoration/main.rs examples/scroll/main.rs examples/scrolling_complete/main.rs examples/semantics_gallery/main.rs examples/simulation/main.rs examples/slivers/main.rs |
| A325 | examples | 1065 | no | examples/snake/game.rs examples/snake/main.rs examples/snake/view.rs examples/studio/commands.rs examples/studio/localization.rs examples/studio/main.rs examples/studio/restoration.rs |
| A326 | examples | 1011 | no | examples/studio/state.rs examples/studio/theme.rs examples/studio/views/bottom_panel.rs examples/studio/views/command_palette.rs examples/studio/views/components.rs |
| A327 | examples | 1050 | no | examples/studio/views/editor.rs examples/studio/views/inspector.rs examples/studio/views/interactive_canvas.rs examples/studio/views/menu_bar.rs examples/studio/views/mod.rs examples/studio/views/project_tree.rs examples/studio/views/settings.rs |
| A328 | examples | 816 | no | examples/studio/views/status_bar.rs examples/text/main.rs examples/text_field/main.rs examples/text_quality/main.rs examples/text_selection/main.rs examples/undecorated_window/main.rs examples/widget_basics/main.rs |
| A329 | incular-android+incular-controls | 885 | no | crates/incular-android/src/lib.rs crates/incular-controls/src/alert_dialog.rs crates/incular-controls/src/app.rs crates/incular-controls/src/autocomplete.rs crates/incular-controls/src/avatar.rs crates/incular-controls/src/scroll_area.rs crates/incular-controls/src/scrollbar.rs crates/incular-controls/src/select.rs |
| A330 | incular-controls+incular-desktop | 874 | no | crates/incular-controls/src/toggle.rs crates/incular-controls/src/toggle_group.rs crates/incular-controls/src/toolbar.rs crates/incular-controls/src/tooltip.rs crates/incular-desktop/src/winit_adapter.rs |
| A331 | incular-devtools+incular-image+incular-ios | 865 | no | tools/incular-devtools/src/views/icons.rs crates/incular-image/src/stream.rs crates/incular-ios/src/lib.rs |
| A332 | incular-material | 886 | no | crates/incular-material/src/inputs/vocabulary.rs crates/incular-material/src/lib.rs crates/incular-material/src/lists.rs crates/incular-material/src/theme/state.rs crates/incular-material/src/theme/vocabulary.rs |
| A333 | incular+incular-navigation | 793 | no | crates/incular-navigation/src/transition_delegate.rs crates/incular/examples/hello.rs crates/incular/src/lib.rs |
| A334 | examples | 448 | no | examples/workbench/main.rs |
