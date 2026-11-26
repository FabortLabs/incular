# Alert, status, form, and nested semantic roles

Five pinned `widgets/basic_test.dart` registrations compare real
mounted semantics:

- **395:** a Dialog and its AlertDialog child retain separate node
  identities and roles.
- **596, 606:** time-sensitive and advisory content retain Alert and
  Status respectively.
- **670:** the list and its item retain their own roles and relationship.
- **693:** an explicitly marked Form is present even when childless.

The retained role enum, semantics debugger, AccessKit projection and
web ARIA mapping distinguish AlertDialog, Alert, Status and Form.
The native projection test checks the four exact AccessKit role values;
the widget test checks the retained tree behind the public Semantics API.
The web role mappings were source-reviewed but no wasm/browser or
physical-device accessibility suite was run.
