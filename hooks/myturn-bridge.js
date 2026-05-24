#!/usr/bin/env node
// myturn-bridge — Stop hook for the MyTurn desktop app.
//
// Fires after each assistant turn. Reads the session JSONL, extracts the last
// assistant message's token usage, and writes ~/.claude/myturn-bridge.json.
// The MyTurn Tauri app watches that file and updates the system tray icon.
//
// Bridge file schema v1:
// {
//   "schema_version": 1,
//   "updated_at": "<ISO-8601>",
//   "session_id": "<uuid>",
//   "model": "<model-id>",
//   "context": {
//     "used_tokens": <number>,
//     "max_tokens": <number>,
//     "percent": <number>        // 0–100, two decimal places
//   },
//   "metrics": {}               // reserved for future TPM/RPM data
// }

'use strict';

const fs            = require('fs');
const path          = require('path');
const os            = require('os');
const { execSync }  = require('child_process');

// All current Claude 4 models have a 200K context window.
// Add entries here as new models ship with different limits.
const CONTEXT_LIMITS = [
  ['claude-opus-4',     200_000],
  ['claude-sonnet-4',   200_000],
  ['claude-haiku-4',    200_000],
  ['claude-3-5-sonnet', 200_000],
  ['claude-3-5-haiku',  200_000],
  ['claude-3-opus',     200_000],
];

function contextLimitForModel(model) {
  if (!model) return 200_000;
  for (const [prefix, limit] of CONTEXT_LIMITS) {
    if (model.startsWith(prefix)) return limit;
  }
  return 200_000; // safe default for unknown future models
}

// Read the last assistant entry from a JSONL transcript.
// Returns { used_tokens, model } or null if nothing found.
function lastUsageFromJSONL(transcriptPath) {
  let raw;
  try { raw = fs.readFileSync(transcriptPath, 'utf8'); }
  catch { return null; }

  const lines = raw.split('\n').filter(l => l.trim());
  // Walk backwards — the last assistant entry is what we want.
  for (let i = lines.length - 1; i >= 0; i--) {
    let entry;
    try { entry = JSON.parse(lines[i]); } catch { continue; }
    if (entry.type !== 'assistant' || !entry.message?.usage) continue;

    const u = entry.message.usage;
    const used =
      (u.input_tokens                 || 0) +
      (u.cache_read_input_tokens      || 0) +
      (u.cache_creation_input_tokens  || 0);

    return { used_tokens: used, model: entry.message.model || null };
  }
  return null;
}

// Atomic write: write to a temp file then rename so the watcher never reads
// a partial JSON document.
function atomicWriteJSON(filePath, data) {
  const tmp = filePath + '.tmp';
  fs.writeFileSync(tmp, JSON.stringify(data, null, 2), 'utf8');
  fs.renameSync(tmp, filePath);
}

// When Claude Code runs inside WSL, os.homedir() returns the WSL Linux home
// (/home/user) but the Windows Tauri app watches the Windows home
// (C:\Users\user\.claude). Detect WSL and redirect to Windows home so both
// sides agree on the bridge file location.
function resolveClaudeDir() {
  if (process.env.CLAUDE_CONFIG_DIR) return process.env.CLAUDE_CONFIG_DIR;
  try {
    const ver = fs.readFileSync('/proc/version', 'utf8');
    if (ver.toLowerCase().includes('microsoft')) {
      const winProfile = execSync('cmd.exe /c "echo %USERPROFILE%"', {
        encoding: 'utf8', timeout: 2000,
      }).trim().replace(/\r?\n.*$/s, '');
      const posix = execSync(`wslpath "${winProfile}"`, {
        encoding: 'utf8', timeout: 2000,
      }).trim();
      if (posix) return path.join(posix, '.claude');
    }
  } catch { /* not WSL or cmd unavailable — fall through */ }
  return path.join(os.homedir(), '.claude');
}

let input = '';
process.stdin.on('data', chunk => { input += chunk; });
process.stdin.on('end', () => {
  let event = {};
  try { event = JSON.parse(input); } catch { /* non-JSON stop event — proceed */ }

  const transcriptPath = event.transcript_path;
  const sessionId      = event.session_id || 'unknown';

  if (!transcriptPath) {
    // No transcript path — nothing to write.
    process.exit(0);
  }

  const result = lastUsageFromJSONL(transcriptPath);
  if (!result) {
    // JSONL exists but no assistant entry yet (e.g., very first message).
    process.exit(0);
  }

  const maxTokens = contextLimitForModel(result.model);
  const percent   = parseFloat(((result.used_tokens / maxTokens) * 100).toFixed(2));

  const bridge = {
    schema_version: 1,
    updated_at:     new Date().toISOString(),
    session_id:     sessionId,
    model:          result.model,
    context: {
      used_tokens: result.used_tokens,
      max_tokens:  maxTokens,
      percent,
    },
    metrics: {},
  };

  const claudeDir  = resolveClaudeDir();
  const bridgePath = path.join(claudeDir, 'myturn-bridge.json');

  try {
    atomicWriteJSON(bridgePath, bridge);
  } catch (e) {
    // Silent fail — never crash a Claude Code session over a metrics write.
  }

  process.exit(0);
});
