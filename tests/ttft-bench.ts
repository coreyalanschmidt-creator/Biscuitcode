// tests/ttft-bench.ts
//
// Phase 5 deliverable + Global AC. Measures first-token latency on
// Anthropic streaming over 20 sequential prompts after a 1-minute
// prewarm; asserts p50 < 500ms, p95 < 1200ms.
//
// Reasoning models are EXEMPT — see Global AC and PROVIDER-TRAIT.md
// § OpenAIProvider. Bench skips any ModelInfo with is_reasoning_model.
//
// Phase 5 referenced in plan.md ACs by THIS path. Don't move.

import { performance } from 'node:perf_hooks';

// ---------- Config ----------

const PREWARM_MS    = 60_000;
const SAMPLE_COUNT  = 20;
const P50_BUDGET_MS = 500;
const P95_BUDGET_MS = 1200;
const PROMPT_TEXT   = 'Reply with the literal three-word string: "fast hello world"';

// ---------- Bench harness ----------
//
// Phase 5 coder fills in `streamFirstToken` against the real
// AnthropicProvider once `chat_stream` is wired. The bench is a
// stand-alone script, NOT vitest — run with:
//   pnpm exec tsx tests/ttft-bench.ts
// (or whatever the project's TS execution method ends up being).

interface Sample {
  index: number;
  ttft_ms: number;
  ok: boolean;
  error?: string;
}

async function streamFirstToken(_modelId: string): Promise<number> {
  // Phase 5 coder:
  //   1. Construct AnthropicProvider with key from libsecret.
  //   2. Start a timer.
  //   3. provider.chat_stream(messages = [{role:user, content:[{type:text, text:PROMPT_TEXT}]}], tools = [], opts = {model, ...}).
  //   4. Iterate the stream until the FIRST ChatEvent::TextDelta arrives.
  //   5. Stop the timer, return elapsed ms.
  throw new Error('TTFT bench: streamFirstToken not yet implemented (Phase 5)');
}

async function prewarm(modelId: string): Promise<void> {
  // Send 1-2 throw-away prompts during the prewarm window so the HTTP/2
  // connection + reqwest keep-alive pool are warm.
  const start = Date.now();
  while (Date.now() - start < PREWARM_MS) {
    try { await streamFirstToken(modelId); } catch { /* swallow */ }
    await sleep(5_000);
  }
}

async function bench(modelId: string): Promise<{ samples: Sample[]; p50: number; p95: number }> {
  const samples: Sample[] = [];
  for (let i = 0; i < SAMPLE_COUNT; i++) {
    try {
      const ttft = await streamFirstToken(modelId);
      samples.push({ index: i, ttft_ms: Math.round(ttft), ok: true });
    } catch (e) {
      samples.push({ index: i, ttft_ms: -1, ok: false, error: String(e) });
    }
  }
  const oks = samples.filter((s) => s.ok).map((s) => s.ttft_ms).sort((a, b) => a - b);
  const p50 = pct(oks, 0.5);
  const p95 = pct(oks, 0.95);
  return { samples, p50, p95 };
}

function pct(sorted: number[], p: number): number {
  if (sorted.length === 0) return Number.POSITIVE_INFINITY;
  const i = Math.min(sorted.length - 1, Math.floor(sorted.length * p));
  return sorted[i];
}

function sleep(ms: number): Promise<void> {
  return new Promise((res) => setTimeout(res, ms));
}

// ---------- Entry point ----------

async function main() {
  const modelId = process.env.BISCUITCODE_BENCH_MODEL ?? 'claude-opus-4-7';
  console.log(`[ttft-bench] model=${modelId}`);
  console.log(`[ttft-bench] prewarming for ${PREWARM_MS / 1000}s …`);
  await prewarm(modelId);

  console.log(`[ttft-bench] sampling ${SAMPLE_COUNT} prompts …`);
  const { samples, p50, p95 } = await bench(modelId);

  console.log('[ttft-bench] samples:');
  for (const s of samples) {
    console.log(`  #${s.index}: ${s.ok ? `${s.ttft_ms}ms` : `FAILED: ${s.error}`}`);
  }
  console.log(`[ttft-bench] p50=${p50}ms (budget ${P50_BUDGET_MS})`);
  console.log(`[ttft-bench] p95=${p95}ms (budget ${P95_BUDGET_MS})`);

  const p50_ok = p50 <= P50_BUDGET_MS;
  const p95_ok = p95 <= P95_BUDGET_MS;
  if (!p50_ok || !p95_ok) {
    console.error(`[ttft-bench] FAILED budget — p50_ok=${p50_ok} p95_ok=${p95_ok}`);
    process.exit(1);
  }
  console.log('[ttft-bench] PASS');
}

if (require.main === module) {
  main().catch((e) => {
    console.error('[ttft-bench] crashed:', e);
    process.exit(2);
  });
}

export { bench, prewarm, streamFirstToken };
