//! Post-registration equivalents of Flutter 3.50.0 arena resolution cases.
//! Incular exposes dispositions instead of callbacks and has no close
//! operation; these tests do not cover Flutter's registration lifecycle.

use incular_gestures::{
    GestureArena, GestureArenaEntry, GestureArenaKey, GestureArenaMember,
    GestureDisposition::{Accepted, Pending, Rejected},
};

const KEY: GestureArenaKey = GestureArenaKey::pointer(1, 4);

fn arena() -> (GestureArena, [GestureArenaMember; 2]) {
    let mut arena = GestureArena::new();
    let first = arena.add(KEY, false);
    let second = arena.add(KEY, false);
    assert_eq!(
        arena.entries(KEY),
        [
            GestureArenaEntry {
                member: first,
                disposition: Pending,
            },
            GestureArenaEntry {
                member: second,
                disposition: Pending,
            },
        ],
    );
    (arena, [first, second])
}

fn assert_winner(
    entries: &[GestureArenaEntry],
    winner: GestureArenaMember,
    loser: GestureArenaMember,
) {
    assert_eq!(entries.len(), 2);
    assert!(
        entries.contains(&GestureArenaEntry {
            member: winner,
            disposition: Accepted
        }) && entries.contains(&GestureArenaEntry {
            member: loser,
            disposition: Rejected
        }),
        "expected {winner:?} accepted and {loser:?} rejected, got {entries:?}",
    );
}

#[test]
fn first_member_can_win_by_accepting() {
    // Upstream test/gestures/arena_test.dart:66, post-close outcome only.
    let (mut arena, [first, second]) = arena();
    assert_winner(&arena.accept(KEY, first), first, second);
}

#[test]
fn only_first_accept_wins_in_registration_order() {
    // Upstream test/gestures/arena_test.dart:114, post-close outcome only.
    let (mut arena, [first, second]) = arena();
    let _ = arena.accept(KEY, first);
    assert_winner(&arena.accept(KEY, second), first, second);
}

#[test]
fn only_first_accept_wins_out_of_registration_order() {
    // Upstream test/gestures/arena_test.dart:125, post-close outcome only.
    let (mut arena, [first, second]) = arena();
    let _ = arena.accept(KEY, second);
    assert_winner(&arena.accept(KEY, first), second, first);
}

#[test]
fn late_rejection_does_not_rewrite_the_resolved_winner() {
    // Authored paired case, not an upstream registration.
    let (mut arena, [winner, loser]) = arena();
    assert_winner(&arena.accept(KEY, winner), winner, loser);
    arena.reject(KEY, winner);
    let after_reject = arena.entries(KEY);
    let after_loser_retries = arena.accept(KEY, loser);
    assert_eq!(after_loser_retries, after_reject);
    assert_winner(&after_reject, winner, loser);
}
