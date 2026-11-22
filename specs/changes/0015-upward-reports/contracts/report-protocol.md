# Report protocol (0015)

One new request on the agent connection (`office-harnessd agent-call`, 0002 contract), and new values in the notification, Needs you, settings, and action record wire forms. The envelope, the proof, and the actor rules are the 0002 ones. User clients are not offered `report.send`: on the office connection it is an unknown type, answered `not_found` like any other unknown type.

## `report.send` (agent caller)

| Field | Type | Required | Meaning |
|---|---|---|---|
| `kind` | `"done"`, `"question"`, `"blocked"`, or `"info"` | yes | What the report says, in the sender's words (A1). |
| `text` | string | yes | 1 to 1,000 Unicode scalar values, not only white space (FR-001). Control and bidirectional format characters are allowed; the server shows them in visible form (below). |
| `for_person` | boolean | no, default `false` | `true` sends the report to the person through Needs you (FR-002). |

No other field is taken. A frame with any other field, such as `target_agent_id`, `agent_id`, `parent_agent_id`, `team_id`, or `to`, is `invalid` and recorded (FR-002). A frame that names `actor` or `caller` is `refused` first, as for every agent request.

**Receiver.** The server reads the tree at the moment of the request. The receiver is the sender's parent, unless the sender has no parent or `for_person` is `true`; then the receiver is the person.

**Visible form.** Before anything else uses it, the text, and the sender's name in the heading, are put in the visible form of the 0014 contract: a C0 character U+0000 to U+001F (line feed and carriage return included) becomes its control picture U+2400 to U+241F, DEL U+007F becomes `␡`, and a C1 character U+0080 to U+009F or a bidirectional format character U+202A to U+202E or U+2066 to U+2069 becomes the six-character text `\uXXXX` (upper-case hex). The length check runs on the text as sent, before this step.

**Line typed into the parent's terminal.**

```
[report from <name> (<agent id>): <kind>] <text>
```

`<name>` is the sender's name in visible form, `<agent id>` its full id, `<kind>` the request's kind, `<text>` the text in visible form. The line holds no line break. It is typed and submitted under the 0012 rules, in the parent's directive queue.

Result:

| Field | Type | Meaning |
|---|---|---|
| `receiver` | `"parent"` or `"person"` | Where the report went. |
| `delivery` | `"submitted"`, `"waiting"`, `"unsubmitted"`, or `"needs_you"` | For a parent, the 0012 outcome; for the person, `"needs_you"`. |
| `position` | number | Only with `"waiting"`: the place in the parent's queue, counted from 1. |

Errors:

| Code | When | Recorded |
|---|---|---|
| `invalid` | A field is not taken; `kind` is missing or not one of the four; `text` is missing, not a string, blank, or longer than 1,000 characters; `for_person` is not a boolean. | Yes, the sender as actor. |
| `refused` | The frame names `actor` or `caller`; the parent is not running; the office hold or an agent hold covers the sender; the reports limit is reached; the parent's queue is full. | Yes, in the sender's record and, when there is a parent, in the parent's. |
| `internal` | The tree or Needs you cannot be read or written. | Yes. |

A refused or withdrawn report counts as 0007 says: a limit refusal counts toward the automatic hold, a hold refusal and a queue-full refusal do not.

## Action record

A new action kind `report`. The entry's text is `<kind>: <text>` with the text in visible form.

| Report | Entry's agent | Actor |
|---|---|---|
| To a parent, accepted | the parent | the sender |
| For the person, accepted | the sender | the sender |
| Withdrawn or unsubmitted (0012) | the sender and the parent, one entry each | the sender |
| Refused | the sender, and the parent when there is one | the sender |

## Notification (amends 0003 `panel-protocol.md`)

A new event `report`. Its `detail` is the report's kind. A report notification is never created as read: exception (b) of 0003/FR-006 does not apply.

## Needs you (amends 0008 `needs-you-protocol.md`)

- An item's `kind` can be `report`; its `detail` is then the report's kind.
- Each item gains `report`: `null`, or `{kind, text, at}` with the latest report for the person (kind, text in visible form, Unix ms). It stays on the item when a later event changes `kind` and `detail` (FR-008).
- An item whose `report` is not `null` is not removed when the agent changes to `starting` or `working`, nor when a hold is released. It is removed when the agent stops or is deleted, when the user opens its terminal or reads its origin notification, and on dismiss.
- Order: `report` sorts in the group of `waiting_for_input`, `bell`, and `auto_hold`.

## Settings (amends 0007 `hold-protocol.md`)

A new key `limit_reports_per_agent`, a whole number from 1 to 1000, default `5`: the reports one agent may send per rolling 60 seconds. A refusal names it "reports sent by one agent".

## Amends

- 0002 `hierarchy-protocol.md`, "New requests (agent caller)": `report.send` joins the allowed types; the not-allowed refusal text names reports among what an agent may do.
