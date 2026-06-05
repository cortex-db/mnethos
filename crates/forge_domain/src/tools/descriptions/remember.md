Persist DURABLE, REUSABLE knowledge about THIS project into long-term memory, so a
future session can recall it. Call this at the END of a task, and ONLY when you
learned something worth remembering. If nothing durable came up, do not call it.

`episodes` is an array — put each distinct piece of knowledge as its own episode
object in the array, and send them all in one call.

Remember vs ignore:
- ✓ conventions, decisions + WHY, non-obvious rules/gotchas, data shapes, commands,
  user preferences — things that would save a future session real work.
- ✗ trivial narration ("read file X"), one-off task mechanics, restating the task.

Each episode has TWO levels — getting the split right is the whole job:

EPISODE = one COHERENT piece of knowledge recalled as a unit. May be a single
fact OR a cohesive rule/convention/scheme that groups several RELATED parts (a
multi-step convention, an enum of related values, a struct's fields). Keep
cohesive knowledge in ONE episode; do NOT shatter a convention into many. But keep
UNRELATED topics as SEPARATE episodes.

Episodes enter a SHARED, cross-project memory graph, recalled independently beside
unrelated projects — so each episode `text` MUST be SELF-CONTAINED and open with
the project identity: what the project IS plus its STABLE name (the repo/package
name). NEVER a filesystem path.
  ✗ "the entry point dispatches via a lookup table"   (which project?)
  ✓ "In <project-name> (<what it is>), <the knowledge, with standalone context>."

CONCEPTS = the ATOMIC sub-units INSIDE an episode. This is where decomposition
happens — ONE concept per atomic relation:
- a multi-step convention → one concept PER step
- an enum (0/1/2 → success/error/usage) → one concept PER value
- a data shape → one concept PER field (or meaningful group)
- a single-relation fact → EXACTLY one concept
A concept is ONE atomic claim, NEVER a paraphrase of the whole episode text.

Each concept has 3-8 anchors. The FIRST anchor is the project name (role
"subject", strength ~0.95). The rest are SINGLE lowercase English lemmas (one word
each, never a phrase) for THIS sub-claim; role ∈ subject|predicate|object|
qualifier|time (subject/object strongest); strength 0..1, varied.

ALWAYS English (text and anchors), regardless of the conversation language —
embeddings live in English semantic space.

Worked example (a DIFFERENT project — copy the STRUCTURE, not the content). Project
"billing-api" (a Go HTTP invoicing service): a 3-stage validation is ONE episode
with THREE atomic concepts (one per stage); a separate money rule is its own
episode with one concept:
{ "episodes": [
  { "text": "In billing-api (a Go HTTP invoicing service), an incoming invoice request is validated in three stages: a JSON-schema check, a tax-rule lookup, then idempotency-key deduplication.",
    "concepts": [
      { "text": "invoice requests are validated against a JSON schema", "confidence": 0.9,
        "anchors": [ {"text":"billing-api","role":"subject","strength":0.95}, {"text":"validate","role":"predicate","strength":0.8}, {"text":"schema","role":"object","strength":0.8} ] },
      { "text": "a tax-rule lookup is one validation stage", "confidence": 0.9,
        "anchors": [ {"text":"billing-api","role":"subject","strength":0.95}, {"text":"tax","role":"object","strength":0.85} ] },
      { "text": "requests are deduplicated by idempotency key", "confidence": 0.9,
        "anchors": [ {"text":"billing-api","role":"subject","strength":0.95}, {"text":"idempotency","role":"object","strength":0.85} ] }
    ] },
  { "text": "In billing-api (a Go HTTP invoicing service), monetary amounts are stored as integer cents, never floats, to avoid rounding errors.",
    "concepts": [
      { "text": "money is stored as integer cents not floats to avoid rounding errors", "confidence": 0.95,
        "anchors": [ {"text":"billing-api","role":"subject","strength":0.95}, {"text":"money","role":"object","strength":0.85}, {"text":"integer","role":"qualifier","strength":0.8} ] }
    ] }
] }
