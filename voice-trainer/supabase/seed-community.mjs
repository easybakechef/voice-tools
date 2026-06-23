// Seed fake community data for testing the Community tab.
//
//   node supabase/seed-community.mjs      (or: npm run db:seed-community)
//
// Creates a few fake users, each with a PUBLIC recording (real, playable
// synthesized audio uploaded to Storage) and some cross-posted feedback.
// Idempotent: re-running first removes any prior @voicetrainer.fake users.
//
// Uses the local service_role key (admin), which bypasses RLS — local dev only.

import { createClient } from '@supabase/supabase-js';

const URL = process.env.SUPABASE_URL ?? 'http://127.0.0.1:54321';
const SERVICE_KEY =
  process.env.SUPABASE_SERVICE_ROLE_KEY ??
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6InNlcnZpY2Vfcm9sZSIsImV4cCI6MTk4MzgxMjk5Nn0.EGIM96RAZx35lJzdJsyH-qQwv8Hdp7fsn3W0YpN81IU';

const FAKE_DOMAIN = '@voicetrainer.fake';

const supabase = createClient(URL, SERVICE_KEY, {
  auth: { autoRefreshToken: false, persistSession: false },
});

// ── Personas ────────────────────────────────────────────────────────────────
const PEOPLE = [
  { name: 'Mara',  email: `mara${FAKE_DOMAIN}`,  title: 'Morning warm-up — humming scales', median: 212, tgt: 82, ratio: 1.48, label: 'Bright' },
  { name: 'Jess',  email: `jess${FAKE_DOMAIN}`,  title: 'Reading practice, take 3',          median: 188, tgt: 64, ratio: 1.31, label: 'Neutral' },
  { name: 'Priya', email: `priya${FAKE_DOMAIN}`, title: 'Conversational pitch test',          median: 228, tgt: 88, ratio: 1.55, label: 'Bright' },
  { name: 'Sam',   email: `sam${FAKE_DOMAIN}`,   title: 'Working on resonance, day 12',       median: 167, tgt: 48, ratio: 1.12, label: 'Dark' },
];

// Feedback to post — [recordingIndex, authorIndex, body]
const FEEDBACK = [
  [0, 2, 'Lovely bright resonance! Your pitch is really consistent here.'],
  [0, 1, 'This sounds great — the warmth in the lower notes is nice.'],
  [1, 0, 'Good progress! Try lifting the end of your sentences a touch more.'],
  [2, 3, 'Wow, super smooth. The forward placement really comes through.'],
  [2, 0, 'Goals 😍 what was your warm-up routine for this?'],
  [3, 2, 'Nice work on the resonance — pitch will follow with practice.'],
];

// ── Synthetic audio: a short sine tone at the persona's median pitch ─────────
function makeWav(freq, seconds = 3, sampleRate = 8000) {
  const n = Math.floor(seconds * sampleRate);
  const buf = Buffer.alloc(44 + n * 2);
  buf.write('RIFF', 0);
  buf.writeUInt32LE(36 + n * 2, 4);
  buf.write('WAVE', 8);
  buf.write('fmt ', 12);
  buf.writeUInt32LE(16, 16);
  buf.writeUInt16LE(1, 20);          // PCM
  buf.writeUInt16LE(1, 22);          // mono
  buf.writeUInt32LE(sampleRate, 24);
  buf.writeUInt32LE(sampleRate * 2, 28);
  buf.writeUInt16LE(2, 32);
  buf.writeUInt16LE(16, 34);
  buf.write('data', 36);
  buf.writeUInt32LE(n * 2, 40);
  for (let i = 0; i < n; i++) {
    // gentle vibrato so it's not a dead tone
    const f = freq * (1 + 0.02 * Math.sin((2 * Math.PI * 5 * i) / sampleRate));
    const s = Math.sin((2 * Math.PI * f * i) / sampleRate) * 0.3;
    buf.writeInt16LE((s * 32767) | 0, 44 + i * 2);
  }
  return buf;
}

// ── Synthetic snapshot data so the cards show realistic stats ────────────────
function synthPitchLog(median, seconds = 3) {
  const out = [];
  for (let t = 0; t < seconds; t += 0.1) {
    out.push({ t: +t.toFixed(2), hz: Math.round(median + Math.sin(t * 3) * 18 + (Math.cos(t * 7) * 6)) });
  }
  return out;
}
function synthFormants(ratio) {
  const f1 = 600;
  return Array.from({ length: 12 }, (_, i) => ({
    f1: Math.round(f1 + Math.sin(i) * 40),
    f2: Math.round(f1 * ratio + Math.cos(i) * 60),
  }));
}

async function cleanup() {
  const { data, error } = await supabase.auth.admin.listUsers({ perPage: 1000 });
  if (error) throw error;
  const stale = data.users.filter((u) => u.email?.endsWith(FAKE_DOMAIN));
  for (const u of stale) {
    const { data: files } = await supabase.storage.from('recordings').list(u.id);
    if (files?.length) {
      await supabase.storage.from('recordings').remove(files.map((f) => `${u.id}/${f.name}`));
    }
    const { data: dsFiles } = await supabase.storage.from('recordings').list(`${u.id}/dataset`);
    if (dsFiles?.length) {
      await supabase.storage.from('recordings').remove(dsFiles.map((f) => `${u.id}/dataset/${f.name}`));
    }
    await supabase.auth.admin.deleteUser(u.id); // cascades recordings + comments + pairs
  }
  if (stale.length) console.log(`Removed ${stale.length} prior fake user(s).`);
}

async function main() {
  console.log(`Seeding community data → ${URL}`);
  await cleanup();

  const created = [];
  for (const p of PEOPLE) {
    const { data: u, error: uErr } = await supabase.auth.admin.createUser({
      email: p.email,
      email_confirm: true,
      user_metadata: { display_name: p.name },
    });
    if (uErr) throw uErr;
    const uid = u.user.id;

    const wav = makeWav(p.median);
    const path = `${uid}/${crypto.randomUUID()}.wav`;
    const { error: upErr } = await supabase.storage
      .from('recordings')
      .upload(path, wav, { contentType: 'audio/wav', upsert: true });
    if (upErr) throw upErr;

    const stats = {
      median: p.median,
      tgtPct: p.tgt,
      pct10: p.median - 30,
      pct90: p.median + 35,
      f2f1Ratio: p.ratio,
      ratioLabel: p.label,
    };
    const { data: rec, error: rErr } = await supabase
      .from('recordings')
      .insert({
        user_id: uid,
        name: p.title,
        duration: 3,
        median_pitch: p.median,
        storage_path: path,
        size_bytes: wav.length,
        visibility: 'public',
        pitch_log: synthPitchLog(p.median),
        formant_data: synthFormants(p.ratio),
        stats,
      })
      .select('id')
      .single();
    if (rErr) throw rErr;

    created.push({ uid, recId: rec.id, name: p.name });
    console.log(`  ✓ ${p.name}: "${p.title}"`);
  }

  for (const [ri, ai, body] of FEEDBACK) {
    const { error } = await supabase.from('comments').insert({
      recording_id: created[ri].recId,
      author_id: created[ai].uid,
      body,
    });
    if (error) throw error;
  }
  console.log(`  ✓ ${FEEDBACK.length} feedback comments`);

  // ── A comparison set with sample pairwise votes (for the 🏆 Rank tab) ──────
  // created[]: 0=Mara, 1=Jess, 2=Priya, 3=Sam. Votes trend Priya > Mara > Jess > Sam.
  const setId = crypto.randomUUID();
  const { error: setErr } = await supabase.from('comparison_sets').insert({
    id: setId,
    creator_id: created[0].uid,
    name: 'Demo dataset — femininity ranking',
    description: 'Four community clips to compare head-to-head.',
  });
  if (setErr) throw setErr;

  const { error: itemErr } = await supabase.from('comparison_items').insert(
    created.map((c) => ({ set_id: setId, recording_id: c.recId })),
  );
  if (itemErr) throw itemErr;

  // [a, b, winner, voter] as indices into created[]
  // [a, b, winner, voter] — each (voter, unordered pair) appears at most once.
  const VOTES = [
    [2, 3, 2, 1], [2, 3, 2, 0], [2, 1, 2, 3], [2, 0, 2, 1],
    [0, 3, 0, 2], [0, 3, 0, 1], [0, 1, 0, 3],
    [1, 3, 1, 2], [1, 3, 1, 0],
    [0, 2, 2, 3], [1, 2, 2, 0], [0, 1, 0, 2],
  ];
  const { error: voteErr } = await supabase.from('comparison_votes').insert(
    VOTES.map(([a, b, w, v]) => ({
      set_id: setId,
      recording_a: created[a].recId,
      recording_b: created[b].recId,
      winner_id: created[w].recId,
      voter_id: created[v].uid,
    })),
  );
  if (voteErr) throw voteErr;
  console.log(`  ✓ comparison set with ${VOTES.length} votes`);

  // ── Public resonance pairs (for the 🎧 Resonance Community tab) ─────────────
  const { data: phrases } = await supabase.from('sample_phrases').select('id').order('sort').limit(3);
  // Each entry: speaker index, phrase index, [bright-voter indices], [deep-voter indices], comment
  const RES = [
    { speaker: 0, phrase: 0, bright: [1, 2], deep: [3], comment: [2, 'The bright take is so much more forward — lovely!'] },
    { speaker: 2, phrase: 1, bright: [0, 1, 3], deep: [],  comment: [1, 'Clear difference, nicely done.'] },
  ];
  for (const r of RES) {
    const speaker = created[r.speaker];
    const pairId = crypto.randomUUID();
    const deepId = crypto.randomUUID();
    const brightId = crypto.randomUUID();
    // Opaque filenames (named by sample id, no label) so labels stay hidden.
    const mk = (sampleId, freq) => ({ path: `${speaker.uid}/dataset/${sampleId}.webm`, wav: makeWav(freq) });
    const deep = mk(deepId, 135);
    const bright = mk(brightId, 235);
    for (const f of [deep, bright]) {
      const { error } = await supabase.storage.from('recordings').upload(f.path, f.wav, { contentType: 'audio/webm', upsert: true });
      if (error) throw error;
    }
    const { error: pErr } = await supabase.from('dataset_pairs')
      .insert({ id: pairId, speaker_id: speaker.uid, phrase_id: phrases[r.phrase].id, visibility: 'public' });
    if (pErr) throw pErr;
    const { error: sErr } = await supabase.from('dataset_samples').insert([
      { id: deepId, pair_id: pairId, speaker_id: speaker.uid, storage_path: deep.path },
      { id: brightId, pair_id: pairId, speaker_id: speaker.uid, storage_path: bright.path },
    ]);
    if (sErr) throw sErr;
    const { error: lErr } = await supabase.from('sample_labels').insert([
      { sample_id: deepId, pair_id: pairId, speaker_id: speaker.uid, label: 'deep' },
      { sample_id: brightId, pair_id: pairId, speaker_id: speaker.uid, label: 'bright' },
    ]);
    if (lErr) throw lErr;
    const votes = [
      ...r.bright.map((v) => ({ pair_id: pairId, voter_id: created[v].uid, chosen_sample_id: brightId })),
      ...r.deep.map((v) => ({ pair_id: pairId, voter_id: created[v].uid, chosen_sample_id: deepId })),
    ];
    if (votes.length) {
      const { error: vErr } = await supabase.from('resonance_votes').insert(votes);
      if (vErr) throw vErr;
    }
    const [ci, body] = r.comment;
    await supabase.from('pair_comments').insert({ pair_id: pairId, author_id: created[ci].uid, body });
  }
  console.log(`  ✓ ${RES.length} public resonance pairs with votes`);

  console.log('\nDone. Open the 💬 Free Form Community and 🎧 Resonance Community tabs to see them.');
}

main().catch((e) => {
  console.error('Seed failed:', e.message ?? e);
  process.exit(1);
});
