use incular_platform::{
    AppLifecycleBinding, AppLifecycleCallback as Callback, AppLifecycleState as State,
    AppLifecycleStateChange,
};
use std::{cell::RefCell, rc::Rc};

fn send(binding: &mut AppLifecycleBinding, value: State) {
    binding
        .handle_platform_message(value.wire_name().as_bytes())
        .unwrap();
}

#[test]
fn an_already_paused_native_view_sets_the_initial_state_without_a_platform_event() {
    let mut binding = AppLifecycleBinding::default();
    assert_eq!(binding.state(), None);
    assert!(
        binding
            .read_initial_state("AppLifecycleState.paused")
            .unwrap()
    );
    assert_eq!(
        binding.state().unwrap().to_string(),
        "AppLifecycleState.paused"
    );
}

#[test]
fn native_initialization_accepts_all_five_states_in_place() {
    for state in State::VALUES {
        let mut binding = AppLifecycleBinding::default();
        assert!(binding.read_initial_state(&state.to_string()).unwrap());
        assert_eq!(binding.state(), Some(state));
    }
}

#[test]
fn state_indices_match_the_state_machine_and_the_wire_enum() {
    assert_eq!(
        State::VALUES,
        [
            State::Detached,
            State::Resumed,
            State::Inactive,
            State::Hidden,
            State::Paused
        ]
    );
    for (index, state) in State::VALUES.into_iter().enumerate() {
        assert_eq!(state as usize, index);
        assert_eq!(state.wire_name().parse::<State>().unwrap(), state);
    }
}

#[test]
fn a_registered_observer_sees_the_synthesized_resumed_edge_before_inactive() {
    let mut binding = AppLifecycleBinding::default();
    binding
        .read_initial_state("AppLifecycleState.detached")
        .unwrap();
    let states = Rc::new(RefCell::new(vec![binding.state().unwrap()]));
    binding.add_observer({
        let states = states.clone();
        move |event| states.borrow_mut().push(event.current)
    });
    assert_eq!(*states.borrow(), [State::Detached]);
    send(&mut binding, State::Inactive);
    assert_eq!(
        *states.borrow(),
        [State::Detached, State::Resumed, State::Inactive]
    );
    send(&mut binding, State::Resumed);
    assert_eq!(
        *states.borrow(),
        [
            State::Detached,
            State::Resumed,
            State::Inactive,
            State::Resumed
        ]
    );
}

#[test]
fn lifecycle_callbacks_follow_all_intermediate_edges_and_detach_is_reversible() {
    use Callback::{Detach, Hide, Inactive, Pause, Restart, Resume, Show};

    let mut binding = AppLifecycleBinding::default();
    binding
        .read_initial_state("AppLifecycleState.detached")
        .unwrap();
    let log = Rc::new(RefCell::new(Vec::new()));
    binding.add_observer({
        let log = log.clone();
        move |event| log.borrow_mut().push(event.callback())
    });
    for (state, last) in [
        (State::Resumed, Resume),
        (State::Inactive, Inactive),
        (State::Hidden, Hide),
        (State::Paused, Pause),
    ] {
        send(&mut binding, state);
        assert_eq!(log.borrow().last(), Some(&last));
    }
    assert_eq!(log.take(), [Resume, Inactive, Hide, Pause]);
    for (state, last) in [
        (State::Hidden, Restart),
        (State::Inactive, Show),
        (State::Resumed, Resume),
    ] {
        send(&mut binding, state);
        assert_eq!(log.borrow().last(), Some(&last));
    }
    assert_eq!(log.take(), [Restart, Show, Resume]);
    send(&mut binding, State::Paused);
    assert_eq!(*log.borrow(), [Inactive, Hide, Pause]);
    send(&mut binding, State::Detached);
    assert_eq!(*log.borrow(), [Inactive, Hide, Pause, Detach]);
    send(&mut binding, State::Resumed);
    assert_eq!(*log.borrow(), [Inactive, Hide, Pause, Detach, Resume]);
    send(&mut binding, State::Paused);
    assert_eq!(
        log.take(),
        [Inactive, Hide, Pause, Detach, Resume, Inactive, Hide, Pause]
    );
    send(&mut binding, State::Resumed);
    assert_eq!(log.take(), [Restart, Show, Resume]);
    send(&mut binding, State::Detached);
    assert_eq!(log.take(), [Inactive, Hide, Pause, Detach]);
}

#[test]
fn repeated_stale_and_invalid_startup_reports_do_not_replay_a_transition() {
    let mut binding = AppLifecycleBinding::default();
    let log = Rc::new(RefCell::new(Vec::new()));
    let observer = binding.add_observer({
        let log = log.clone();
        move |event| log.borrow_mut().push(event)
    });
    assert!(!binding.read_initial_state("").unwrap());
    assert!(binding.read_initial_state("unrecognized").is_err());
    assert!(binding.state().is_none());
    send(&mut binding, State::Paused);
    assert_eq!(
        log.take(),
        [AppLifecycleStateChange {
            previous: None,
            current: State::Paused
        }]
    );
    assert!(
        !binding
            .read_initial_state("AppLifecycleState.resumed")
            .unwrap()
    );
    send(&mut binding, State::Paused);
    assert!(log.borrow().is_empty());
    assert!(binding.remove_observer(observer));
    send(&mut binding, State::Resumed);
    assert_eq!(binding.state(), Some(State::Resumed));
    assert!(log.borrow().is_empty());
}
