use std::{cell::RefCell, rc::Rc};

use incular_config::Locale;
use incular_platform::{
    DefaultSpellCheckService, FLUTTER_SPELL_CHECK_CHANNEL, FlutterSpellCheckReply,
    SpellCheckService, SuggestionSpan,
    message_codecs::{MessageValue as V, PlatformException, StandardMethodCodec as Codec},
};

type Transport = Rc<RefCell<Vec<(Vec<u8>, FlutterSpellCheckReply)>>>;

fn service() -> (DefaultSpellCheckService, Transport) {
    let calls: Transport = Rc::default();
    let service = DefaultSpellCheckService::with_flutter_method_channel({
        let calls = calls.clone();
        move |channel, packet, reply| {
            assert_eq!(channel, FLUTTER_SPELL_CHECK_CHANNEL);
            assert_eq!(channel, "flutter/spellcheck");
            calls.borrow_mut().push((packet, reply));
        }
    });
    (service, calls)
}

fn request(
    service: &DefaultSpellCheckService,
    locale: &str,
    text: &str,
) -> Rc<RefCell<Option<Option<Vec<SuggestionSpan>>>>> {
    let outcome = Rc::new(RefCell::new(None));
    let locale: Locale = locale.parse().unwrap();
    service.fetch_spell_check_suggestions(
        &locale,
        text,
        Box::new({
            let outcome = outcome.clone();
            move |result| *outcome.borrow_mut() = Some(result)
        }),
    );
    outcome
}

fn entry(start: i64, end: i64, choices: &[&str]) -> V {
    V::Map(vec![
        (V::String("startIndex".into()), V::Int(start)),
        (V::String("endIndex".into()), V::Int(end)),
        (
            V::String("suggestions".into()),
            V::List(
                choices
                    .iter()
                    .map(|text| V::String((*text).into()))
                    .collect(),
            ),
        ),
    ])
}

#[test]
fn default_checker_encodes_the_locale_and_converts_non_ascii_suggestion_offsets() {
    let (service, sent) = service();
    let result = request(&service, "en-GB", "😀 tset");
    assert!(result.borrow().is_none());
    let (packet, reply) = sent.borrow_mut().pop().unwrap();
    let call = Codec::decode_method_call(&packet).unwrap();
    assert_eq!(call.method, "SpellCheck.initiateSpellCheck");
    assert_eq!(
        call.arguments,
        V::List(vec![V::String("en-GB".into()), V::String("😀 tset".into())])
    );

    reply(Some(
        Codec::encode_success_envelope(&V::List(vec![entry(3, 7, &["test", "sets", "set"])]))
            .unwrap(),
    ));
    assert_eq!(
        *result.borrow(),
        Some(Some(vec![SuggestionSpan::new(
            5..9,
            ["test", "sets", "set"]
        )]))
    );
    assert_eq!(
        service.last_saved_results().unwrap().spell_checked_text,
        "😀 tset"
    );

    let clean = request(&service, "fr-CA", "test");
    let (_, reply) = sent.borrow_mut().pop().unwrap();
    reply(Some(
        Codec::encode_success_envelope(&V::List(vec![])).unwrap(),
    ));
    assert_eq!(*clean.borrow(), Some(Some(vec![])));
    let last = service.last_saved_results();

    let busy = request(&service, "fr-CA", "busy");
    let (_, reply) = sent.borrow_mut().pop().unwrap();
    reply(Some(
        Codec::encode_error_envelope(&PlatformException {
            code: "busy".into(),
            message: None,
            details: V::Null,
            stacktrace: None,
        })
        .unwrap(),
    ));
    assert_eq!(*busy.borrow(), Some(None));
    assert_eq!(service.last_saved_results(), last);
}

#[test]
fn invalid_utf16_offsets_or_missing_replies_never_publish_a_partial_answer() {
    let (service, sent) = service();
    let replies = [
        None,
        Some(Codec::encode_success_envelope(&V::Null).unwrap()),
        Some(
            Codec::encode_success_envelope(&V::List(vec![
                entry(2, 3, &["a"]),
                entry(1, 2, &["split surrogate"]),
            ]))
            .unwrap(),
        ),
        Some(Codec::encode_success_envelope(&V::List(vec![entry(2, 4, &["past end"])])).unwrap()),
        Some(Codec::encode_success_envelope(&V::List(vec![entry(3, 2, &["reversed"])])).unwrap()),
    ];
    for bytes in replies {
        let result = request(&service, "en", "😀a");
        let (_, reply) = sent.borrow_mut().pop().unwrap();
        reply(bytes);
        assert_eq!(*result.borrow(), Some(None));
    }
    assert!(service.last_saved_results().is_none());
}
