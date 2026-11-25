# Keyboard services

Flutter reference: pinned Flutter 3.50.0 checkout.

## Control-key label classification

The exact `services/keyboard_key_test.dart:50` registration passed with
the same six control labels and four non-control labels. Two focused
Rust tests also check C0/C1 endpoints, Unicode joiners and emoji, and
multi-character labels. The predicate classifies one label rather than
changing native keyboard dispatch or text filtering.
