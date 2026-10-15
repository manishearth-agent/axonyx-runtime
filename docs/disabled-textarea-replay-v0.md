# Disabled Textarea Replay V0

Native action 422 recovery can opt in a disabled textarea without enabling it:

```html
<form data-ax-retain-fields="title,content_document" action="...">
  <textarea name="content_document" data-ax-retain-disabled="true"
    hidden="true" disabled="true"></textarea>
</form>
```

Replay still requires the matching action/route, a bounded URL-encoded POST,
unambiguous field names and the server-rendered form's explicit allowlist.
Secret/transport names and secret autocomplete controls remain excluded.
Disabled inputs/selects and unmarked textareas keep the old behavior.

For an opted-in textarea with a submitted value, the runtime sets text children
and `data-ax-retained="true"`. Normal HTML rendering escapes that text; this is
not HTML injection or submitted JSON embedded in a script. The disabled attribute
is preserved, so no-JS submissions are not enabled. Values remain excluded from
the serialized/debug form result. The 64 KiB request-wide budget is unchanged.

An editor may consume the retained text only after validating its document. It
must not silently replace invalid retained input with stale stored content.
This does not bypass server write validation or repair persisted rows. The
marker distinguishes a native retry from ordinary field values during remount.

This behavior requires axonyx-runtime 0.6.9 or newer. Registry consumers must
resolve that runtime version; installing a CLI alone does not change a locked
application dependency. See runtime-0.6.9.md for the release scope and gates.
