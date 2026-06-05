use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use forge_app::{AgentService, EnvironmentInfra, Services};
use forge_domain::{
    ChatCompletionMessageFull, Context, ContextMessage, Conversation, ConversationId, EndPayload,
    EventData, EventHandle, ResponseFormat, ResultStreamExt, Role,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::config::MemoryConfig;
use crate::domain::{MemoryAnchor, MemoryConcept, MemoryEpisode};
use crate::session::project_session_key;

/// PoC consolidation hook for the Mnethos memory layer.
///
/// On conversation end it asks the SAME model the agent is using (the default
/// selected model for now; a dedicated memory model comes later) to extract
/// durable memory *episodes* — the caller-parsed contract the memory service
/// expects (text + concepts[] + anchors[]) — from what the agent just did.
///
/// It DUMPS the extracted episodes to `.mnethos/episodes/` for inspection and
/// prompt-tuning, then writes them to the memory server over gRPC
/// (CreateEpisode). The write is a no-op when no `memory.json` is present, so
/// normal runs are unaffected. Opt-in via `MNETHOS_MEMORY`. Never mutates the
/// conversation, never fails the turn.
#[derive(Clone)]
pub struct MemoryConsolidationHandler<S> {
    services: Arc<S>,
}

impl<S> MemoryConsolidationHandler<S> {
    pub fn new(services: Arc<S>) -> Self {
        Self { services }
    }
}

/// Basic consolidation prompt — the thing we are tuning. Kept inline so it is
/// easy to iterate; the model returns JSON matching [`ConsolidationResponse`].
const CONSOLIDATION_PROMPT: &str = r#"You are the memory-consolidation step of an AI coding agent.

You are given a JSON record of a task the agent just completed: a `project`
block (the codebase's `name` and `files`), the user's request, the assistant's
messages, tool calls/results, and file-operation metrics. Extract the DURABLE,
REUSABLE knowledge worth remembering — what was done, how, and especially WHY.
Works for ANY knowledge (code, infra, config, product, process, a decision, a
gotcha); the rules are about SHAPE, not topic.

The output has TWO levels — getting the split right is the whole job:

EPISODE = one COHERENT piece of knowledge to recall as a unit. It may be a
single fact, OR a cohesive rule / convention / scheme / data-shape that
naturally groups several RELATED parts (a multi-step convention, an enum of
related values, a struct's fields). Keep such cohesive knowledge in ONE episode
— do NOT shatter one convention into many episodes. But do NOT merge UNRELATED
knowledge: independent topics (e.g. a command-dispatch mechanism vs a storage
rule) are SEPARATE episodes. Group what belongs together; separate what doesn't.
An episode's `text` MAY be a richer, multi-part sentence — that is fine.

CONCEPTS = the ATOMIC sub-units INSIDE an episode. This is where decomposition
happens, and concepts MUST be atomic — break the episode into ONE concept per
atomic relation:
- a multi-step convention   → one concept PER step
- an enum (e.g. 0/1/2 → success/error/usage)  → one concept PER value
- a data-shape with fields  → one concept PER field (or meaningful group)
- a single-relation fact    → EXACTLY one concept
A concept is ONE atomic claim — NEVER a paraphrase of the whole episode `text`.

SHARED GRAPH — episodes go into ONE shared, cross-project memory graph (one
user's whole history now, a global graph later), recalled INDEPENDENTLY beside
unrelated projects. So:
- The episode `text` MUST be SELF-CONTAINED and open with the project identity:
  what the project IS (purpose/nature) AND its STABLE name — use `project.name`
  VERBATIM (the repo/package name). NEVER a filesystem path or directory
  (machine-specific, useless in shared memory).
  ✗ "the entry point dispatches via a lookup table"  (which project?)
  ✓ "In <project-name> (<what it is>), <the knowledge, with standalone context>."
- Prefer DURABLE knowledge: conventions, decisions + rationale, non-obvious
  rules/gotchas, structure — NOT trivial narration ("read file X").

Each concept:
- text: one atomic sub-claim — NOT the project prefix, NOT the whole episode.
- confidence: 0..1.
- anchors[]: 3-8. The FIRST is `project.name` (role subject, strength ~0.95) —
  it scopes the fact (a proper-noun slug is fine; it becomes the project's own
  anchor). The rest are SINGLE lowercase English lemmas (one word each, never a
  phrase) for THIS sub-claim; role ∈ subject|predicate|object|qualifier|time
  (subject/object strongest); strength 0..1, VARIED.

WORKED EXAMPLE (a DIFFERENT, unrelated project — copy the STRUCTURE, not the
content). Project `name` "billing-api" (a Go HTTP invoicing service). A 3-stage
validation is ONE cohesive episode with THREE atomic concepts (one per stage); a
separate integer-money rule is ONE episode with ONE concept:
{ "episodes": [
  { "text": "In billing-api (a Go HTTP invoicing service), an incoming invoice request is validated in three stages: a JSON-schema check, a tax-rule lookup, then idempotency-key deduplication.",
    "concepts": [
      { "text": "invoice requests are validated against a JSON schema", "confidence": 0.9,
        "anchors": [ {"text":"billing-api","role":"subject","strength":0.95}, {"text":"validate","role":"predicate","strength":0.8}, {"text":"schema","role":"object","strength":0.8} ] },
      { "text": "a tax-rule lookup is one validation stage", "confidence": 0.9,
        "anchors": [ {"text":"billing-api","role":"subject","strength":0.95}, {"text":"tax","role":"object","strength":0.85}, {"text":"lookup","role":"predicate","strength":0.7} ] },
      { "text": "requests are deduplicated by idempotency key", "confidence": 0.9,
        "anchors": [ {"text":"billing-api","role":"subject","strength":0.95}, {"text":"idempotency","role":"object","strength":0.85}, {"text":"dedup","role":"predicate","strength":0.75} ] }
    ] },
  { "text": "In billing-api (a Go HTTP invoicing service), monetary amounts are stored as integer cents, never floats, to avoid rounding errors.",
    "concepts": [
      { "text": "money is stored as integer cents not floats to avoid rounding errors", "confidence": 0.95,
        "anchors": [ {"text":"billing-api","role":"subject","strength":0.95}, {"text":"money","role":"object","strength":0.85}, {"text":"integer","role":"qualifier","strength":0.8}, {"text":"rounding","role":"qualifier","strength":0.6} ] }
    ] }
] }
Note: cohesive knowledge (the 3 stages) stays ONE episode but decomposes into one
concept PER stage; the independent money rule is its own episode; a single
relation is one concept. Anchors are single lowercase lemmas; project name first.

Now do the same for the ACTUAL project below. Return ONLY JSON:
{ "episodes": [ { "text": "...", "concepts": [ { "text": "...", "confidence": 0.9,
  "anchors": [ { "text": "lemma", "role": "subject", "strength": 0.9 } ] } ] } ] }

If there is nothing durable worth remembering, return { "episodes": [] }."#;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct AnchorOut {
    text: String,
    /// subject|predicate|object|qualifier|time
    role: String,
    strength: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct ConceptOut {
    text: String,
    confidence: f32,
    anchors: Vec<AnchorOut>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct EpisodeOut {
    text: String,
    concepts: Vec<ConceptOut>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(title = "consolidation")]
struct ConsolidationResponse {
    episodes: Vec<EpisodeOut>,
}

#[async_trait]
impl<S: Services + EnvironmentInfra<Config = forge_config::ForgeConfig>>
    EventHandle<EventData<EndPayload>> for MemoryConsolidationHandler<S>
{
    async fn handle(
        &self,
        event: &EventData<EndPayload>,
        conversation: &mut Conversation,
    ) -> anyhow::Result<()> {
        if std::env::var_os("MNETHOS_MEMORY").is_none() {
            return Ok(());
        }
        if let Err(err) = self.consolidate(event, conversation).await {
            warn!(error = %err, "memory-consolidation: failed (non-fatal)");
        }
        Ok(())
    }
}

impl<S: Services + EnvironmentInfra<Config = forge_config::ForgeConfig>>
    MemoryConsolidationHandler<S>
{
    async fn consolidate(
        &self,
        event: &EventData<EndPayload>,
        conversation: &Conversation,
    ) -> anyhow::Result<()> {
        let input = build_input_json(conversation)?;

        let schema = schemars::schema_for!(ConsolidationResponse);
        // NB: no explicit temperature — newer models (e.g. claude-opus-4-8)
        // REJECT the `temperature` param ("deprecated for this model", HTTP 400).
        // The JSON-schema response_format already constrains output shape.
        let ctx = Context::default()
            .conversation_id(ConversationId::generate())
            .add_message(ContextMessage::system(CONSOLIDATION_PROMPT))
            .add_message(ContextMessage::user(input, Some(event.model_id.clone())))
            .response_format(ResponseFormat::JsonSchema(Box::new(schema)));

        // Retry transient API failures (mirrors the core agent loop; this
        // out-of-loop call otherwise drops on a single flaky-API blip).
        let retry_cfg = self.services.get_config()?.retry.unwrap_or_default();
        let full = crate::retry::with_retry(&retry_cfg, || {
            let ctx = ctx.clone();
            async move {
                let stream = self
                    .services
                    .chat_agent(&event.model_id, ctx, Some(event.agent.provider.clone()))
                    .await?;
                stream.into_full(false).await
            }
        })
        .await?;
        let ChatCompletionMessageFull { content, usage, .. } = full;

        let parsed = serde_json::from_str::<ConsolidationResponse>(&content).ok();

        let observed_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        let episode_count = parsed.as_ref().map(|p| p.episodes.len()).unwrap_or(0);
        let dump = serde_json::json!({
            "observed_at_ms": observed_at_ms,
            "agent_id": serde_json::to_value(&event.agent.id)?,
            "model_id": serde_json::to_value(&event.model_id)?,
            "parsed_ok": parsed.is_some(),
            "episode_count": episode_count,
            // Token cost of THIS consolidation call (sends the whole conversation
            // to the model to extract episodes) — the write-side memory overhead,
            // separate from the agent task's own usage.
            "consolidation_usage": serde_json::to_value(&usage).ok(),
            "episodes": parsed.as_ref().map(|p| serde_json::to_value(&p.episodes).ok()),
            // Always keep the raw model output so we can tune the prompt even
            // when JSON parsing fails.
            "raw_model_output": content,
        });

        let dir = PathBuf::from(".mnethos").join("episodes");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}-{}.json", conversation.id, observed_at_ms));
        std::fs::write(&path, serde_json::to_string_pretty(&dump)?)?;

        info!(
            path = %path.display(),
            episodes = episode_count,
            parsed_ok = parsed.is_some(),
            "memory-consolidation: episodes extracted and dumped"
        );

        // Write episodes to the memory server (no-op when memory.json is absent).
        if let Some(response) = parsed {
            let episodes: Vec<MemoryEpisode> =
                response.episodes.iter().map(to_memory_episode).collect();
            if !episodes.is_empty() {
                // Same key the retrieval hook recalls under (cwd basename).
                let session_key = project_session_key(&self.services.get_environment().cwd);
                match MemoryConfig::load() {
                    Ok(Some(cfg)) => {
                        let client = cfg.client();
                        let mut written = 0usize;
                        for episode in episodes {
                            match client.create_episode(&session_key, episode).await {
                                Ok(_) => written += 1,
                                Err(err) => {
                                    warn!(error = %err, "memory-consolidation: write episode failed")
                                }
                            }
                        }
                        info!(
                            written,
                            session_key = %session_key,
                            "memory-consolidation: episodes written to memory"
                        );
                    }
                    Ok(None) => {
                        info!("memory-consolidation: no memory.json; dump only")
                    }
                    Err(err) => {
                        warn!(error = %err, "memory-consolidation: invalid memory.json")
                    }
                }
            }
        }

        Ok(())
    }
}

/// Builds the consolidation input: the conversation trimmed to non-system
/// messages (the user task + assistant turns + tool calls/results) plus the
/// file-operation metrics. System prompts are dropped as noise.
fn build_input_json(conversation: &Conversation) -> anyhow::Result<String> {
    let messages = conversation.context.as_ref().map(|ctx| {
        ctx.messages
            .iter()
            .filter(|m| !m.has_role(Role::System))
            .collect::<Vec<_>>()
    });

    // Project-identity block: episodes enter a shared cross-project graph, so
    // the model needs to know which codebase this is to make each episode
    // self-contained. Derived from the files the agent touched.
    let mut paths: Vec<String> = conversation
        .metrics
        .files_accessed
        .iter()
        .cloned()
        .collect();
    paths.sort();
    let project_dir = common_dir(&paths);
    // Project NAME = the project root's basename (e.g. the repo folder name) — a
    // stable, portable identifier used to scope episodes; NOT the absolute path.
    let project_name = project_dir.rsplit('/').next().unwrap_or_default().to_string();
    let files: Vec<String> = paths
        .iter()
        .map(|p| {
            p.strip_prefix(&project_dir)
                .unwrap_or(p)
                .trim_start_matches('/')
                .to_string()
        })
        .collect();

    let input = serde_json::json!({
        // Project name (basename) + relative file names — NOT the absolute
        // directory: a local path is machine-specific and must never enter the
        // shared memory graph.
        "project": { "name": project_name, "files": files },
        "task_title": serde_json::to_value(&conversation.title)?,
        "messages": serde_json::to_value(&messages)?,
        "metrics": serde_json::to_value(&conversation.metrics)?,
    });

    Ok(serde_json::to_string_pretty(&input)?)
}

/// Longest common directory prefix of the given absolute paths ("" if none).
fn common_dir(paths: &[String]) -> String {
    let Some(first) = paths.first() else {
        return String::new();
    };
    let mut prefix = first.clone();
    for p in &paths[1..] {
        while !p.starts_with(&prefix) {
            prefix.pop();
            if prefix.is_empty() {
                return String::new();
            }
        }
    }
    match prefix.rfind('/') {
        Some(i) => prefix[..i].to_string(),
        None => prefix,
    }
}

/// Converts the model's parsed episode into the domain `MemoryEpisode` sent to
/// the memory service (the contracts are 1:1).
fn to_memory_episode(e: &EpisodeOut) -> MemoryEpisode {
    MemoryEpisode {
        text: e.text.clone(),
        concepts: e
            .concepts
            .iter()
            .map(|c| MemoryConcept {
                text: c.text.clone(),
                confidence: c.confidence,
                anchors: c
                    .anchors
                    .iter()
                    .map(|a| MemoryAnchor {
                        text: a.text.clone(),
                        role: a.role.clone(),
                        strength: a.strength,
                    })
                    .collect(),
            })
            .collect(),
    }
}
