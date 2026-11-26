//! Post-registration equivalents of Flutter 3.50.0 arena resolution cases.
//! Incular exposes dispositions rather than callbacks; sole-member closure
//! is supported here without modeling the entire binding lifecycle.

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

#[test]
fn pending_simultaneous_members_can_still_reject_and_full_streams_can_cancel() {
    let mut arena = GestureArena::new();
    let winner = arena.add(KEY, true);
    let undecided = arena.add(KEY, true);
    let after_win = arena.accept(KEY, winner);
    assert_eq!(after_win[0].disposition, Accepted);
    assert_eq!(after_win[1].disposition, Pending);
    arena.reject(KEY, winner);
    assert_eq!(arena.entries(KEY), after_win);

    arena.reject(KEY, undecided);
    assert_winner(&arena.entries(KEY), winner, undecided);
    let canceled = arena.cancel(KEY);
    assert_eq!(canceled.len(), 2);
    assert!(
        canceled
            .iter()
            .all(|entry| entry.disposition == incular_gestures::GestureDisposition::Cancelled)
    );
    assert!(arena.entries(KEY).is_empty());
}

#[test]
fn closing_waits_for_contenders_and_only_publishes_a_default_win_once() {
    let mut arena = GestureArena::new();
    let key = GestureArenaKey::pointer(0, 77);
    let first = arena.add(key, false);
    let second = arena.add(key, false);
    assert!(arena.close_sole_member(key).is_empty());
    arena.reject(key, second);
    assert!(arena.close_sole_member(key).iter().any(|entry| {
        entry.member == first && entry.disposition == Accepted
    }));
    assert!(arena.close_sole_member(key).is_empty());
}
