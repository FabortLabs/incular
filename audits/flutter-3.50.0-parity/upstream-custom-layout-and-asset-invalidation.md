# Custom single-child layout and asset invalidation

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

Four original CustomSingleChildLayout tests pass on Flutter.
Control parent/child dimensions (:100) and a real size change (:144)
match. The newest delegate should receive shouldRelayout against
the previous delegate (:121). Incular calls it on the previous
delegate with the replacement and does not perform the requested
layout. Initial receiver logs [2,3]; replacements see no call, and
the expected new layout does not occur. Zero-area layout (:172)
matches through an inert required Rust child and fixed-size
delegate, not identical notifier/no-child APIs; partial.
See `flutter_custom_single_layout_parity`.

AssetBundle clear and targeted eviction (:116, :147) clear cached
plain and pre-parsed text as intended; evict leaves the unselected
key live. Both original registrations pass on Flutter and their
Rust text subsets pass. Flutter's structured-binary branch has no
public Rust equivalent and is excluded from these partial results.
See `flutter350_asset_cache_invalidation_parity`.
