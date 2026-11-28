# SensitiveContent host and capture-policy restoration

Reference: Flutter 3.50.0 `packages/flutter/test/widgets/sensitive_content_host_test.dart`.
The upstream test binary mocks the native channel with an existing
`autoSensitive` policy. Incular's test drives keyed widgets through full retained
application frames and records the real window operations against a supported
in-memory platform adapter with the same observed baseline. This does not
establish OS screenshot, screen recording, or Android API behavior.

- Lines 53–1017: 35 scenarios cover one or more siblings, all three policy
  priorities, removal, the remaining most protective policy, and restoration of
  the policy the adapter reported before the override. The command is emitted
  only when the active policy changes.
- Lines 1051–1311: all 9 single-widget policy transitions are exercised on the
  same retained key, including unchanged settings.
- Lines 2060, 2079, 2101: nested requests aggregate across ancestors. Line 2060
  actually mounts the sensitive parent outside the automatic child, despite
  its upstream description.
- Lines 2123 and 2146: final nested policies agree, but Flutter temporarily
  sends `notSensitive` before its inner child registers. Incular resolves the
  subtree atomically and avoids that weaker transient command; marked partial.
- Line 2037: the upstream disjunction exits its loop before mounting anything.
  Independent native probes cover three concrete settings but the vacuous
  source registration is not counted as paired.

Unscoped Incular content no longer initializes native capture policy to a
value the application did not request, or overwrites a manual host setting on
subsequent frames. The backend may report its pre-existing policy explicitly.
Seven adjacent operation-bridge tests passed, including typed native
unsupported responses for explicitly requested operations.

## Multiple retained registrations

The second original-matrix run covers all 18 keyed changes at lines
1343–1995, with one unchanged sibling in each mounted case. Four cases
(1787, 1829, 1871, 1914) are partial because Flutter briefly emits its
first weaker registration on initial mount, whereas Incular sends only
the final window policy. The other 14 were paired, checking both the
resulting policy and the complete command sequence.

Incular now maintains policy counts at mount, update, and unmount instead
of scanning every live element during frame submission. A focused regression
moves a live global key between parents without remounting, removes it,
then replaces a sensitivity scope with an unrelated environment value.
The six sensitivity tests, seven adjacent native-operations tests, and two
autofill-lifecycle neighbors passed.
