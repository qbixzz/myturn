'use strict';
// Tests for ~/.claude/hooks/myturn-bridge.js
// Run: node --test hook-tests/bridge-hook.test.js
//
// Strategy: test the hook through its public interface — the bridge file it
// writes. Each test spawns the hook as a child process with controlled stdin
// (the Stop event JSON) and a temp JSONL transcript, then asserts on the
// bridge file contents.

const { test }       = require('node:test');
const assert         = require('node:assert/strict');
const { spawnSync }  = require('node:child_process');
const fs             = require('node:fs');
const os             = require('node:os');
const path           = require('node:path');

const HOOK = path.join(os.homedir(), '.claude', 'hooks', 'myturn-bridge.js');
const NODE = process.execPath;

// ── Helpers ──────────────────────────────────────────────────────────────────

function makeTmpDir() {
    return fs.mkdtempSync(path.join(os.tmpdir(), 'myturn-test-'));
}

function writeJSONL(dir, lines) {
    const p = path.join(dir, 'session.jsonl');
    fs.writeFileSync(p, lines.map(l => JSON.stringify(l)).join('\n') + '\n');
    return p;
}

function runHook(stdinData, env = {}) {
    return spawnSync(NODE, [HOOK], {
        input:   JSON.stringify(stdinData),
        env:     { ...process.env, ...env },
        timeout: 5000,
        encoding: 'utf8',
    });
}

function assistantEntry(model, usage) {
    return {
        type:    'assistant',
        message: { model, usage },
    };
}

// ── Tests ────────────────────────────────────────────────────────────────────

// Tracer bullet: hook writes bridge file with correct percent
test('writes bridge file with correct context_window_usage percent', () => {
    const dir          = makeTmpDir();
    const bridgePath   = path.join(dir, 'myturn-bridge.json');
    const transcriptPath = writeJSONL(dir, [
        assistantEntry('claude-sonnet-4-6', {
            input_tokens:                100,
            cache_read_input_tokens:     20000,
            cache_creation_input_tokens: 10000,
            output_tokens:               200,
        }),
    ]);

    runHook(
        { transcript_path: transcriptPath, session_id: 'test-session-1' },
        { CLAUDE_CONFIG_DIR: dir }
    );

    const bridge = JSON.parse(fs.readFileSync(bridgePath, 'utf8'));
    // used = 100 + 20000 + 10000 = 30100
    // percent = 30100 / 200000 * 100 = 15.05
    assert.equal(bridge.context.used_tokens, 30100);
    assert.equal(bridge.context.max_tokens, 200000);
    assert.ok(Math.abs(bridge.context.percent - 15.05) < 0.01);
});

// used_tokens = input + cache_read + cache_creation
test('used_tokens sums all three token fields', () => {
    const dir          = makeTmpDir();
    const bridgePath   = path.join(dir, 'myturn-bridge.json');
    const transcriptPath = writeJSONL(dir, [
        assistantEntry('claude-sonnet-4-6', {
            input_tokens:                1,
            cache_read_input_tokens:     21523,
            cache_creation_input_tokens: 11285,
            output_tokens:               132,
        }),
    ]);

    runHook(
        { transcript_path: transcriptPath, session_id: 'test-session-2' },
        { CLAUDE_CONFIG_DIR: dir }
    );

    const bridge = JSON.parse(fs.readFileSync(bridgePath, 'utf8'));
    assert.equal(bridge.context.used_tokens, 1 + 21523 + 11285);
});

// Missing transcript_path → no bridge file written, no crash
test('missing transcript_path does not write bridge file and exits cleanly', () => {
    const dir        = makeTmpDir();
    const bridgePath = path.join(dir, 'myturn-bridge.json');

    const result = runHook(
        { session_id: 'no-transcript' },
        { CLAUDE_CONFIG_DIR: dir }
    );

    assert.equal(result.status, 0, `hook crashed: ${result.stderr}`);
    assert.ok(!fs.existsSync(bridgePath), 'bridge file should not exist');
});

// JSONL with no assistant entries → no bridge file written
test('jsonl with no assistant entries does not write bridge file', () => {
    const dir          = makeTmpDir();
    const bridgePath   = path.join(dir, 'myturn-bridge.json');
    const transcriptPath = writeJSONL(dir, [
        { type: 'user', message: { content: 'hello' } },
    ]);

    const result = runHook(
        { transcript_path: transcriptPath, session_id: 'no-assistant' },
        { CLAUDE_CONFIG_DIR: dir }
    );

    assert.equal(result.status, 0);
    assert.ok(!fs.existsSync(bridgePath), 'bridge file should not exist');
});

// claude-sonnet-4-6 → max_tokens = 200000
test('model claude-sonnet-4-6 uses 200000 as context_limit', () => {
    const dir          = makeTmpDir();
    const bridgePath   = path.join(dir, 'myturn-bridge.json');
    const transcriptPath = writeJSONL(dir, [
        assistantEntry('claude-sonnet-4-6', {
            input_tokens:                500,
            cache_read_input_tokens:     0,
            cache_creation_input_tokens: 0,
            output_tokens:               100,
        }),
    ]);

    runHook(
        { transcript_path: transcriptPath, session_id: 'test-model' },
        { CLAUDE_CONFIG_DIR: dir }
    );

    const bridge = JSON.parse(fs.readFileSync(bridgePath, 'utf8'));
    assert.equal(bridge.context.max_tokens, 200000);
    assert.equal(bridge.model, 'claude-sonnet-4-6');
});

// Bridge file has schema_version = 1
test('bridge file has schema_version 1', () => {
    const dir          = makeTmpDir();
    const bridgePath   = path.join(dir, 'myturn-bridge.json');
    const transcriptPath = writeJSONL(dir, [
        assistantEntry('claude-sonnet-4-6', {
            input_tokens: 100, cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0, output_tokens: 50,
        }),
    ]);

    runHook(
        { transcript_path: transcriptPath, session_id: 'test-schema' },
        { CLAUDE_CONFIG_DIR: dir }
    );

    const bridge = JSON.parse(fs.readFileSync(bridgePath, 'utf8'));
    assert.equal(bridge.schema_version, 1);
});
